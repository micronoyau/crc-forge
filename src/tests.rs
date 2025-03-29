#[cfg(test)]
use crate::{CRC32, Generator, Polynomial};

#[test]
pub fn test_single_letter() {
    let crc = CRC32::new(Generator {
        polynomial: Polynomial::Normal(0x04c11db7),
        init_xor: 0xffffffff,
        final_xor: 0xffffffff,
    });
    assert_eq!(crc.compute(b"a".iter()), 0xe8b7be43);
}

#[test]
pub fn test_empty() {
    let crc = CRC32::new(Generator {
        polynomial: Polynomial::Normal(0x04c11db7),
        init_xor: 0xffffffff,
        final_xor: 0xffffffff,
    });
    assert_eq!(crc.compute(b"".iter()), 0);
}

#[test]
pub fn test_hello() {
    let crc = CRC32::new(Generator {
        polynomial: Polynomial::Normal(0x04c11db7),
        init_xor: 0xffffffff,
        final_xor: 0xffffffff,
    });
    assert_eq!(crc.compute(b"hello, world!".iter()), 0x58988d13);
}
