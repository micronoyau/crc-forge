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
    m: u32,                          // m = CRC(0)
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

        // Precompute CRC(0) = (I mod G) ^ F
        let i = Polynomial::from(PolynomialRepr::Reverse(generator.i));
        let f = Polynomial::from(PolynomialRepr::Reverse(generator.f));
        let m: Polynomial<u32> = ((i * xn + f.into()) % generator.g_full).try_into()?;
        let m = m.repr();

        Ok(Self {
            generator,
            table,
            xn_inv,
            m,
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

    /// Compute CRC32 checksum for `data`.
    /// Input data must be in little-endian representation.
    pub fn checksum<'a, T>(&self, data: T) -> u32
    where
        T: Iterator<Item = &'a u8>,
    {
        // CRC is remainder of data times X^N where N = deg(generator)
        // In CRC32, N = 32 bits
        let mut data = data.chain([0u8; 4].iter());

        // Initialize CRC register
        let mut register: u32 = 0;
        for _ in 0..4 {
            let b = data.next().unwrap(); // unwrap ok because just added 4 elements
            register >>= 8;
            register |= u32::from(*b) << 24;
        }
        register ^= self.generator.i;

        while let Some(b) = data.next() {
            self.step(&mut register, *b);
        }

        register ^ self.generator.f
    }

    /// Compute 4-byte suffix to `data` so that resulting CRC is `target_crc`.
    pub fn compute_suffix<'a, T>(&self, data: T, target_crc: u32) -> [u8; 4]
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
        let res = res + c + f;
        res.repr().to_le_bytes()
    }
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
    use crate::{CRC32, Generator};

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
        let newc = crc.checksum(data.iter());
        assert_eq!(crc.checksum([0u8; 4].iter()), crc.m);
        assert_eq!(newc, crc.m);
    }

    #[test]
    pub fn test_suffix() {
        let crc = CRC32::new(Generator::default()).unwrap();
        let data = b"lorem ipsum";
        let ct = 0x42424242;
        let suffix = crc.compute_suffix(data.iter(), ct);
        let data_suffixed = [data, &suffix[..]].concat();
        let newc = crc.checksum(data_suffixed.iter());
        assert_eq!(newc, ct);
    }
}
