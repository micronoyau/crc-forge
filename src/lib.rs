use std::{fmt::Debug, u32};

mod error;
use error::{CRCResult, Error};

pub mod math;
use math::{Polynomial, PolynomialRepr, reverse_u32};

const CRC32_LOOKUP_SIZE: usize = 0x100;

/// CRC32 properties: generator polynomial and init and final xor values.
pub struct CRC32Properties {
    g: u32, // Generator polynomial with highest term is stripped (implicitely of degree 32), as usually given
    i: u32, // I, value initially xored with input
    f: u32, // F, value eventually xored with output
}

impl Default for CRC32Properties {
    fn default() -> Self {
        Self {
            g: 0x04c11db7u32,
            i: 0xffffffff,
            f: 0xffffffff,
        }
    }
}

/// Fast CRC implementation over simple polynomial operations.
pub struct CRC32 {
    props: CRC32Properties,          // Generator G
    g: Polynomial<u64>,              // Generator polynomial, not strippped
    table: [u32; CRC32_LOOKUP_SIZE], // 8 bit lookup side
    xn_inv: Polynomial<u32>,         // (X^N)-1 mod G
}

/// Compute register mask in reverse table at index `index`.
/// `index` should be given in little endian representation.
/// `g` is the (stripped) CRC generator polynomial.
fn precompute_table(index: u8, g: u32) -> u32 {
    let mut register = u32::from(index);
    for _ in 0..8 {
        let div = register & 1;
        register >>= 1;
        if div == 1 {
            // Since the polynomial is given without the leading 1, subtraction is done AFTER shift
            register ^= g;
        }
    }
    register
}

impl CRC32 {
    /// Create CRC32 instance with generator polynomial `generator`.
    pub fn new(props: CRC32Properties) -> CRCResult<Self> {
        // Precompute table
        let mut table = [0u32; CRC32_LOOKUP_SIZE];
        for i in 0..CRC32_LOOKUP_SIZE {
            table[i] = precompute_table(i as u8, reverse_u32(props.g));
        }

        // Compute full generator polynomial
        let xn = Polynomial::from(PolynomialRepr::Normal(1u64 << 32));
        let g = xn + Polynomial::from(PolynomialRepr::Normal(props.g)).into();

        // Precompute (X^N)^-1 mod G
        let xn_inv = xn.inv_mod(g)?.try_into()?;

        Ok(Self {
            props,
            g,
            table,
            xn_inv,
        })
    }

    /// Perform a single one-byte division step using table.
    fn step(&self, register: &mut u32, next_byte: u8) {
        let index = *register & 0xff;
        let mask = self.table[index as usize];
        *register >>= 8;
        *register |= u32::from(next_byte) << 24;
        *register ^= mask;
    }

    /// Compute fast remainder of data by generator polynomial using precomputed tables.
    /// `data` is the data to process, `register` is the CRC register to start with.
    fn fast_rem<'a, T>(&self, mut data: T, register: u32) -> u32
    where
        T: Iterator<Item = &'a u8>,
    {
        let mut reg = 0u32;

        // First populate CRC register
        for _ in 0..4 {
            // Append leading zeros to data polynomial if less than 4 bytes
            match data.next() {
                Some(&b) => {
                    reg >>= 8;
                    reg |= u32::from(b) << 24
                }
                None => break,
            }
        }

        // XOR with initial register value
        reg ^= register;

        // Step through division
        while let Some(b) = data.next() {
            self.step(&mut reg, *b);
        }

        reg
    }

    /// Compute CRC32 checksum for `data`.
    /// Input data must be in little-endian representation.
    pub fn checksum<'a, T>(&self, data: T) -> u32
    where
        T: Iterator<Item = &'a u8>,
    {
        // CRC is remainder of data times X^N where N = deg(G)
        // In CRC32, N = 32 bits
        let data = data.chain([0u8; 4].iter());
        self.fast_rem(data, self.props.i) ^ self.props.f
    }

    /// Helper to compute polynomial `p` mod generator.
    fn generator_remainder<T>(&self, p: T) -> Polynomial<u32>
    where
        T: Into<Polynomial<u128>>,
    {
        // Cant fail because `self.g` is of degree 32 so remainder is at most of degree 31
        (p.into() % self.g).try_into().unwrap()
    }

    /// Compute suffix polynomial to `data` so that resulting CRC is `target_crc`.
    fn compute_suffix_polynomial<'a, T>(&self, data: T, target_crc: u32) -> Polynomial<u32>
    where
        T: Iterator<Item = &'a u8>,
    {
        let c = Polynomial::from(PolynomialRepr::Reverse(self.checksum(data)));
        let cp = Polynomial::from(PolynomialRepr::Reverse(target_crc));
        let f = Polynomial::from(PolynomialRepr::Reverse(self.props.f));
        // Compute inserted data: (C' + F) X^N^-1 + C + F mod G
        let res = self.generator_remainder((cp + f) * self.xn_inv);
        res + c + f
    }

    /// Compute 4-byte suffix to `data` so that resulting CRC is `target_crc`.
    pub fn compute_suffix<'a, T>(&self, data: T, target_crc: u32) -> [u8; 4]
    where
        T: Iterator<Item = &'a u8>,
    {
        self.compute_suffix_polynomial(data, target_crc)
            .repr()
            .to_le_bytes()
    }

    /// Compute inserted polynomial at offset `offset` of `data` so that resulting CRC is `target_crc`.
    fn compute_inserted_polynomial<'a, T>(
        &self,
        data: T,
        offset: usize,
        target_crc: u32,
    ) -> CRCResult<Polynomial<u32>>
    where
        T: Iterator<Item = &'a u8> + Clone,
    {
        let end_offset = data
            .clone()
            .count()
            .checked_sub(offset)
            .ok_or(Error::OverflowError(None))?;

        // Split data
        let suffix = data.clone().skip(offset);
        let suffix_poly = Polynomial::from(PolynomialRepr::Reverse(self.fast_rem(suffix, 0)));

        // X^M and (X^M)^-1
        let xm: Polynomial<u32> = Polynomial::from(PolynomialRepr::Normal(2u64))
            .pow(8 * end_offset as u64, self.g)
            .try_into()
            .unwrap();
        let xm_inv: Polynomial<u32> = Polynomial::<u64>::from(xm)
            .inv_mod(self.g)
            .unwrap()
            .try_into()
            .unwrap();

        // Compute inserted polynomial: ((C' + F) X^N^-1 + C + F + suffix (1 + X^N)) X^M^-1 mod G
        let inserted = self.compute_suffix_polynomial(data, target_crc);
        let inserted = inserted + suffix_poly;
        let inserted = inserted
            + self.generator_remainder(
                Polynomial::from(PolynomialRepr::Normal(1u64 << 32)) * suffix_poly,
            );
        let inserted = self.generator_remainder(inserted * xm_inv);

        Ok(inserted)
    }

    /// Compute inserted polynomial at offset `offset` of `data` so that resulting CRC is `target_crc`.
    pub fn compute_inserted<'a, T>(
        &self,
        data: T,
        offset: usize,
        target_crc: u32,
    ) -> CRCResult<[u8; 4]>
    where
        T: Iterator<Item = &'a u8> + Clone,
    {
        Ok(self
            .compute_inserted_polynomial(data, offset, target_crc)?
            .repr()
            .to_le_bytes())
    }
}

impl Debug for CRC32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Generator = {:?}",
            Polynomial::from(PolynomialRepr::Normal(1u64 << 32)) + self.g.into()
        )?;
        writeln!(f, "Table:")?;
        for i in 0..0x20 {
            write!(f, "{:02x}: ", i << 3)?;
            for j in 0..8 {
                write!(f, "{:08x} ", self.table[i * 8 + j])?;
            }
            writeln!(f, "")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        CRC32, CRC32Properties,
        math::{Polynomial, PolynomialRepr},
    };

    #[test]
    pub fn test_table() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        assert_eq!(
            crc.table,
            [
                0x0, 0x77073096, 0xee0e612c, 0x990951ba, 0x76dc419, 0x706af48f, 0xe963a535,
                0x9e6495a3, 0xedb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x9b64c2b, 0x7eb17cbd,
                0xe7b82d07, 0x90bf1d91, 0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d,
                0x6ddde4eb, 0xf4d4b551, 0x83d385c7, 0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec,
                0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5, 0x3b6e20c8, 0x4c69105e, 0xd56041e4,
                0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b, 0x35b5a8fa, 0x42b2986c,
                0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59, 0x26d930ac,
                0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
                0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab,
                0xb6662d3d, 0x76dc4190, 0x1db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x6b6b51f,
                0x9fbfe4a5, 0xe8b8d433, 0x7807c9a2, 0xf00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb,
                0x86d3d2d, 0x91646c97, 0xe6635c01, 0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e,
                0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457, 0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea,
                0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65, 0x4db26158, 0x3ab551ce,
                0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb, 0x4369e96a,
                0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
                0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409,
                0xce61e49f, 0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81,
                0xb7bd5c3b, 0xc0ba6cad, 0xedb88320, 0x9abfb3b6, 0x3b6e20c, 0x74b1d29a, 0xead54739,
                0x9dd277af, 0x4db2615, 0x73dc1683, 0xe3630b12, 0x94643b84, 0xd6d6a3e, 0x7a6a5aa8,
                0xe40ecf0b, 0x9309ff9d, 0xa00ae27, 0x7d079eb1, 0xf00f9344, 0x8708a3d2, 0x1e01f268,
                0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7, 0xfed41b76, 0x89d32be0,
                0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5, 0xd6d6a3e8,
                0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
                0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef,
                0x4669be79, 0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703,
                0x220216b9, 0x5505262f, 0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7,
                0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d, 0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x26d930a,
                0x9c0906a9, 0xeb0e363f, 0x72076785, 0x5005713, 0x95bf4a82, 0xe2b87a14, 0x7bb12bae,
                0xcb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0xbdbdf21, 0x86d3d2d4, 0xf1d4e242,
                0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777, 0x88085ae6,
                0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
                0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d,
                0x3e6e77db, 0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5,
                0x47b2cf7f, 0x30b5ffe9, 0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605,
                0xcdd70693, 0x54de5729, 0x23d967bf, 0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94,
                0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d
            ]
        );
    }

    #[test]
    pub fn test_fast_rem() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let data = 0x421234012430091u64;
        let data_poly = Polynomial::from(PolynomialRepr::Reverse(data));
        let rem_fast = crc.fast_rem(data.to_le_bytes().iter(), 0);
        let rem_poly: Polynomial<u32> = (data_poly % crc.g).try_into().unwrap();
        assert_eq!(rem_fast, rem_poly.repr());
    }

    #[test]
    pub fn test_crc_equivalent() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let xn = Polynomial::from(PolynomialRepr::Normal(1u64 << 32));
        let i = Polynomial::from(PolynomialRepr::Reverse(crc.props.i));
        let f = Polynomial::from(PolynomialRepr::Reverse(crc.props.f));

        let data = 0x8f3b86b3726f6cu64;
        let data_poly = Polynomial::from(PolynomialRepr::Reverse(data));
        let data_bytes = data.to_le_bytes();
        println!("data = {:x?} = {:?}", data_bytes, data);

        // First compute checksum using tables
        let c = crc.checksum(data_bytes.iter());
        println!(
            "Real CRC = 0x{:x} = {:?}",
            c,
            Polynomial::from(PolynomialRepr::Reverse(c))
        );

        // Then compute CRC using polynomial multiplication
        let cp: Polynomial<u32> = ((data_poly * xn) % crc.g).try_into().unwrap();
        let cp = cp
            + ((i * Polynomial::from(PolynomialRepr::Normal(2)).pow(8 * 8, crc.g)) % crc.g)
                .try_into()
                .unwrap();
        let cp = cp + f;

        println!("Polynomial CRC = {:?} = 0x{:x}", cp, cp.repr());
        assert_eq!(c, cp.repr());
    }

    #[test]
    pub fn test_single_letter() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        assert_eq!(crc.checksum(b"a".iter()), 0xe8b7be43);
    }

    #[test]
    pub fn test_empty() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        assert_eq!(crc.checksum(b"".iter()), 0);
    }

    #[test]
    pub fn test_hello() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        assert_eq!(crc.checksum(b"hello, world!".iter()), 0x58988d13);
    }

    #[test]
    pub fn test_identity_suffix() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let data = b"lorem ipsum";
        let c = crc.checksum(data.iter());
        let data: Vec<u8> = data.to_owned().into_iter().chain(c.to_le_bytes()).collect();
        let new_c = crc.checksum(data.iter());
        assert_eq!(new_c, crc.checksum([0u8; 4].iter()));
    }

    #[test]
    pub fn test_suffix() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let data = b"lorem ipsum";
        let target_c = 0x42424242;
        let suffix = crc.compute_suffix(data.iter(), target_c);
        let data_suffixed = [data, &suffix[..]].concat();
        let new_c = crc.checksum(data_suffixed.iter());
        assert_eq!(new_c, target_c);
    }

    #[test]
    pub fn test_small_suffix() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let data = b"d";
        let target_c = 0x42424242;
        let suffix = crc.compute_suffix(data.iter(), target_c);
        let data_suffixed = [data, &suffix[..]].concat();
        let new_c = crc.checksum(data_suffixed.iter());
        assert_eq!(new_c, target_c);
    }

    #[test]
    pub fn test_insertion_end() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let data = b"lorem ipsum";
        let target_c = 0x42424242;
        let offset = 8;
        let inserted = crc.compute_inserted(data.iter(), offset, target_c).unwrap();
        let edited_data = [
            &data[..offset as usize],
            &inserted[..],
            &data[offset as usize..],
        ]
        .concat();
        println!("edited = {:?}", edited_data);
        let new_c = crc.checksum(edited_data.iter());
        assert_eq!(new_c, target_c);
    }

    #[test]
    pub fn test_insertion_start() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let data = b"lorem ipsum";
        let target_c = 0x42424242;
        let offset = 2;
        let inserted = crc.compute_inserted(data.iter(), offset, target_c).unwrap();
        let edited_data = [
            &data[..offset as usize],
            &inserted[..],
            &data[offset as usize..],
        ]
        .concat();
        println!("edited = {:?}", edited_data);
        let new_c = crc.checksum(edited_data.iter());
        assert_eq!(new_c, target_c);
    }

    #[test]
    pub fn test_insertion_middle() {
        let crc = CRC32::new(CRC32Properties::default()).unwrap();
        let data = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
        let target_c = 0x42424242;
        let offset = 42;
        let inserted = crc.compute_inserted(data.iter(), offset, target_c).unwrap();
        let edited_data = [
            &data[..offset as usize],
            &inserted[..],
            &data[offset as usize..],
        ]
        .concat();
        println!("edited = {:?}", edited_data);
        let new_c = crc.checksum(edited_data.iter());
        assert_eq!(new_c, target_c);
    }
}
