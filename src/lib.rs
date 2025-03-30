use std::{fmt::Debug, u32};

mod error;

pub mod math;
use math::{Polynomial, PolynomialRepr};

const CRC32_LOOKUP_SIZE: usize = 0x100;

/// Generator with polynomial and init and final xor values.
pub struct Generator {
    polynomial: Polynomial<u32>, // Generator polynomial (highest term is implicitly of degree 32)
    init_xor: u32,               // Value initially XORed with input
    final_xor: u32,              // Value eventually XORed with output
}

impl Default for Generator {
    fn default() -> Self {
        Self {
            polynomial: Polynomial::from(PolynomialRepr::Normal(0x04c11db7)),
            init_xor: 0xffffffff,
            final_xor: 0xffffffff,
        }
    }
}

pub struct CRC32 {
    generator: Generator,
    table: [u32; CRC32_LOOKUP_SIZE], // 8 bit lookup side
}

/// Compute register mask in forward table at index `index`.
/// `index` should be given in little endian representation
/// `generator` is the CRC generator polynomial
fn precompute_table(index: u8, generator: &Generator) -> u32 {
    let mut register = u32::from(index);
    let polynomial: u32 = generator.polynomial.0;
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
    pub fn new(generator: Generator) -> Self {
        let mut table = [0u32; CRC32_LOOKUP_SIZE];
        for i in 0..CRC32_LOOKUP_SIZE {
            table[i] = precompute_table(i as u8, &generator);
        }
        Self { generator, table }
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
    pub fn compute<'a, T>(&self, data: T) -> u32
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
        register ^= self.generator.init_xor;

        while let Some(b) = data.next() {
            self.step(&mut register, *b);
        }

        register ^ self.generator.final_xor
    }
}

impl Debug for CRC32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Generator = {:?}",
            Polynomial::from(PolynomialRepr::Normal(1u64 << 32)) + self.generator.polynomial.into()
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
        let crc = CRC32::new(Generator::default());
        assert_eq!(crc.compute(b"a".iter()), 0xe8b7be43);
    }

    #[test]
    pub fn test_empty() {
        let crc = CRC32::new(Generator::default());
        assert_eq!(crc.compute(b"".iter()), 0);
    }

    #[test]
    pub fn test_hello() {
        let crc = CRC32::new(Generator::default());
        assert_eq!(crc.compute(b"hello, world!".iter()), 0x58988d13);
    }
}
