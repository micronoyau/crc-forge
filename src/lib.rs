use std::{fmt::Debug, u32};

mod error;

pub mod math;
use error::CRCResult;
use math::{Polynomial, PolynomialRepr};

const CRC32_LOOKUP_SIZE: usize = 0x100;

/// Generator with polynomial and init and final xor values.
pub struct Generator {
    g: Polynomial<u32>, // Generator polynomial (highest term is implicitly of degree 32)
    g_full: Polynomial<u64>, // Generator polynomial with highest term
    i: u32,             // I, value initially xored with input
    f: u32,             // F, value eventually xored with output
}

impl Default for Generator {
    fn default() -> Self {
        let g = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let g_full = Polynomial::from(PolynomialRepr::Normal(1u64 << 32)) + g.into();
        Self {
            g,
            g_full,
            i: 0xffffffff,
            f: 0xffffffff,
        }
    }
}

pub struct CRC32 {
    generator: Generator,            // Generator G
    table: [u32; CRC32_LOOKUP_SIZE], // 8 bit lookup side
    xn_inv: Polynomial<u32>,         // (X^N)-1 mod G
}

/// Compute register mask in forward table at index `index`.
/// `index` should be given in little endian representation
/// `generator` is the CRC generator polynomial
fn precompute_table(index: u8, generator: &Generator) -> u32 {
    let mut register = u32::from(index);
    let polynomial: u32 = generator.g.repr();
    for _ in 0..8 {
        let div = register & 1;
        register >>= 1;
        if div == 1 {
            // Since the polynomial is given without the leading 1 (term of degree 32
            // is not given), subtraction is done AFTER shift
            register ^= polynomial;
        }
    }
    register
}

impl CRC32 {
    /// Create CRC32 instance with generator polynomial `generator`.
    pub fn new(generator: Generator) -> CRCResult<Self> {
        // Precompute table
        let mut table = [0u32; CRC32_LOOKUP_SIZE];
        for i in 0..CRC32_LOOKUP_SIZE {
            table[i] = precompute_table(i as u8, &generator);
        }

        // Precompute (X^N)^-1 mod G
        let xn = Polynomial::from(PolynomialRepr::Normal(1u64 << 32));
        let xn_inv = xn.inv_mod(generator.g_full)?.try_into()?;

        Ok(Self {
            generator,
            table,
            xn_inv,
        })
    }

    /// Perform a single one-byte step using table.
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
        for _ in 0..4 {
            reg >>= 8;
            if let Some(b) = data.next() {
                reg |= u32::from(*b) << 24;
            }
        }
        reg ^= register;

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
        self.fast_rem(data, self.generator.i) ^ self.generator.f
    }

    /// Compute suffix polynomial to `data` so that resulting CRC is `target_crc`.
    fn compute_suffix_core<'a, T>(&self, data: T, target_crc: u32) -> Polynomial<u32>
    where
        T: Iterator<Item = &'a u8>,
    {
        let c = Polynomial::from(PolynomialRepr::Reverse(self.checksum(data)));
        let cp = Polynomial::from(PolynomialRepr::Reverse(target_crc));
        let f = Polynomial::from(PolynomialRepr::Reverse(self.generator.f));
        // Cant fail because g_full is of degree 32 so remainder fits in 32 bits
        let res: Polynomial<u32> = (((cp + f) * self.xn_inv) % self.generator.g_full)
            .try_into()
            .unwrap();
        res + c + f
    }

    /// Compute 4-byte suffix to `data` so that resulting CRC is `target_crc`.
    pub fn compute_suffix<'a, T>(&self, data: T, target_crc: u32) -> [u8; 4]
    where
        T: Iterator<Item = &'a u8>,
    {
        self.compute_suffix_core(data, target_crc)
            .repr()
            .to_le_bytes()
    }

    /// Compute 4-byte value inserted at offset `offset` from end in `data` so that resulting CRC is `target_crc`.
    pub fn compute_inserted<'a, T>(&self, data: T, offset: usize, target_crc: u32) -> [u8; 4]
    where
        T: Iterator<Item = &'a u8> + Clone,
    {
        // Split data in two
        let prefix = data.clone().take(data.clone().count() - offset);
        let suffix = data.clone().skip(data.clone().count() - offset);

        let c = Polynomial::from(PolynomialRepr::Reverse(self.checksum(data.clone())));
        let cp = Polynomial::from(PolynomialRepr::Reverse(target_crc));
        let f = Polynomial::from(PolynomialRepr::Reverse(self.generator.f));
        // Cant fail because g_full is of degree 32 so remainder fits in 32 bits
        let res: Polynomial<u32> = (((cp + f) * self.xn_inv) % self.generator.g_full)
            .try_into()
            .unwrap();
        let res = res + c + f;

        // Compute (X^M)^-1 mod G
        let xm = Polynomial::from(PolynomialRepr::Normal(2u64))
            .pow(offset as u64, self.generator.g_full);
        let xm_inv = xm.inv_mod(self.generator.g_full).unwrap();

        // Compute prod = (1+X^N) * suffix mod G
        let suffix_shifted = suffix.clone().chain([0u8; 4].iter()); // suffix * X^N
        let prod = self.fast_rem(suffix, 0) ^ self.fast_rem(suffix_shifted, 0); // suffix * X^N + suffix mod G
        let prod = Polynomial::from(PolynomialRepr::Reverse(prod));

        let res = res + prod;

        // Cant fail because g_full is of degree 32 so remainder fits in 32 bits
        let res: Polynomial<u32> = ((res * xm_inv) % self.generator.g_full).try_into().unwrap();
        res.repr().to_le_bytes()
    }

    // /// Compute 4-byte value inserted at offset `offset` from end in `data` so that resulting CRC is `target_crc`.
    // pub fn compute_inserted<'a, T>(&self, data: T, offset: u64, target_crc: u32) -> [u8; 4]
    // where
    //     T: Iterator<Item = &'a u8> + Clone,
    // {
    //     let res = self.compute_suffix_core(
    //         data.clone().take(data.clone().count() - offset as usize),
    //         target_crc,
    //     );

    //     // Compute (X^M)^-1 mod G
    //     let xm = Polynomial::from(PolynomialRepr::Normal(2u64)).pow(offset, self.generator.g_full);
    //     let xm_inv = xm.inv_mod(self.generator.g_full).unwrap();

    //     // Compute prod = (1+X^N) * suffix mod G
    //     let suffix = data.clone().skip(data.clone().count() - offset as usize);
    //     let suffix_shifted = suffix.clone().chain([0u8; 4].iter()); // suffix * X^N
    //     let prod = self.fast_rem(suffix, 0) ^ self.fast_rem(suffix_shifted, 0); // suffix * X^N + suffix mod G
    //     let prod = Polynomial::from(PolynomialRepr::Reverse(prod));

    //     let res = res + prod;

    //     // Cant fail because g_full is of degree 32 so remainder fits in 32 bits
    //     let res: Polynomial<u32> = ((res * xm_inv) % self.generator.g_full).try_into().unwrap();
    //     res.repr().to_le_bytes()
    // }
}

impl Debug for CRC32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Generator = {:?}",
            Polynomial::from(PolynomialRepr::Normal(1u64 << 32)) + self.generator.g.into()
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
        CRC32, Generator,
        math::{Polynomial, PolynomialRepr},
    };

    #[test]
    pub fn test_fast_rem() {
        let crc = CRC32::new(Generator::default()).unwrap();
        let data = 0x421234012430091u64;
        let data_poly = Polynomial::from(PolynomialRepr::Reverse(data));
        let rem_fast = crc.fast_rem(data.to_le_bytes().iter(), 0);
        let rem_poly: Polynomial<u32> = (data_poly % crc.generator.g_full).try_into().unwrap();
        assert_eq!(rem_fast, rem_poly.repr());
    }

    #[test]
    pub fn test_single_letter() {
        let crc = CRC32::new(Generator::default()).unwrap();
        assert_eq!(crc.checksum(b"a".iter()), 0xe8b7be43);
    }

    #[test]
    pub fn test_empty() {
        let crc = CRC32::new(Generator::default()).unwrap();
        assert_eq!(crc.checksum(b"".iter()), 0);
    }

    #[test]
    pub fn test_hello() {
        let crc = CRC32::new(Generator::default()).unwrap();
        assert_eq!(crc.checksum(b"hello, world!".iter()), 0x58988d13);
    }

    #[test]
    pub fn test_identity_suffix() {
        let crc = CRC32::new(Generator::default()).unwrap();
        let data = b"lorem ipsum";
        let c = crc.checksum(data.iter());
        let data: Vec<u8> = data.to_owned().into_iter().chain(c.to_le_bytes()).collect();
        let new_c = crc.checksum(data.iter());
        assert_eq!(new_c, crc.checksum([0u8; 4].iter()));
    }

    #[test]
    pub fn test_suffix() {
        let crc = CRC32::new(Generator::default()).unwrap();
        let data = b"lorem ipsum";
        let target_c = 0x42424242;
        let suffix = crc.compute_suffix(data.iter(), target_c);
        let data_suffixed = [data, &suffix[..]].concat();
        let new_c = crc.checksum(data_suffixed.iter());
        assert_eq!(new_c, target_c);
    }

    #[test]
    pub fn test_placeholder() {
        let crc = CRC32::new(Generator::default()).unwrap();
        let data = b"lorem ipsum";
        let target_c = 0x42424242;
        let offset = 1;
        let inserted = crc.compute_inserted(data.iter(), offset, target_c);
        let edited_data = [
            &data[..data.len() - offset as usize],
            &inserted[..],
            &data[data.len() - offset as usize..],
        ]
        .concat();
        println!("edited = {:?}", edited_data);
        let new_c = crc.checksum(edited_data.iter());
        assert_eq!(new_c, target_c);
    }
}
