use crate::{
    error::Error,
    math::{self, Polynomial, PolynomialRepr},
};
// use crate::{CRC32, Generator, Polynomial};

// #[test]
// pub fn test_single_letter() {
//     let crc = CRC32::new(Generator {
//         polynomial: Polynomial::from(PolynomialRepr::Normal(0x04c11db7)),
//         init_xor: 0xffffffff,
//         final_xor: 0xffffffff,
//     });
//     assert_eq!(crc.compute(b"a".iter()), 0xe8b7be43);
// }

// #[test]
// pub fn test_empty() {
//     let crc = CRC32::new(Generator {
//         polynomial: Polynomial::from(PolynomialRepr::Normal(0x04c11db7)),
//         init_xor: 0xffffffff,
//         final_xor: 0xffffffff,
//     });
//     assert_eq!(crc.compute(b"".iter()), 0);
// }

// #[test]
// pub fn test_hello() {
//     let crc = CRC32::new(Generator {
//         polynomial: Polynomial::from(PolynomialRepr::Normal(0x04c11db7)),
//         init_xor: 0xffffffff,
//         final_xor: 0xffffffff,
//     });
//     assert_eq!(crc.compute(b"hello, world!".iter()), 0x58988d13);
// }

#[test]
pub fn test_poly_invmod_simple() {
    let p: Polynomial<u64> = Polynomial::from(PolynomialRepr::Normal(0x04c11db7));
    let a: Polynomial<u64> = Polynomial::from(PolynomialRepr::Normal(0x2));

    println!("P = {:?}", p);
    println!("P = {:x}", p.0);
    println!("A = {:?}", a);
    println!("A = {:x}", a.0);

    println!("P/A = {:?}", p / a);

    // let a_inv = a.invmod(Polynomial::try_from(p).unwrap()).unwrap();
    // println!("A^-1 = {:?}", a_inv);
    // let prod = a * a_inv.into();
    // let prod = prod % p;
    // println!("A^-1 * A mod P = {:?}", prod);

    let a: () = Err(Error::NonInvertibleError).unwrap();

    // assert_eq!(
    //     math::modular_inverse(0x100000000, 0x04c11db7).unwrap(),
    //     0xcbf1acda
    // );
}

#[test]
pub fn test_poly_invmod() {
    // let crc = CRC32::new(Generator {
    //     polynomial: Polynomial::from(PolynomialRepr::Normal(0x04c11db7))
    //         .try_into()
    //         .unwrap(),
    //     init_xor: 0xffffffff,
    //     final_xor: 0xffffffff,
    // });

    let generator: Polynomial<u64> = Polynomial::from(PolynomialRepr::Normal(0x04c11db7));
    let xn: Polynomial<u64> = Polynomial::from(PolynomialRepr::Normal(0x100000000));
    let xn_inv: Polynomial<u64> = Polynomial::from(PolynomialRepr::Normal(0xcbf1acda));

    println!("G = {:?}", generator);
    println!("G = {:x}", generator.0);
    println!("X^N = {:?}", xn);
    println!("X^N = {:x}", xn.0);
    println!("G + X^N = {:?}", generator + xn);
    println!("G * X^N = {:?}", generator * xn);

    let prod = Polynomial::<u64>::try_from(xn * xn_inv).unwrap();
    println!("X^N * (X^N)^-1 = {:?}", prod);
    let prod = prod % (generator + xn);
    println!("X^N * (X^N)^-1 mod (G + X^N) = {:?}", prod);

    let xn_inv = xn.invmod(Polynomial::try_from(generator).unwrap());
    println!("(X^N)^-1 mod (G + X^N) = {:?}", xn_inv);

    let a: () = Err(Error::NonInvertibleError).unwrap();

    // assert_eq!(
    //     math::modular_inverse(0x100000000, 0x04c11db7).unwrap(),
    //     0xcbf1acda
    // );
}
