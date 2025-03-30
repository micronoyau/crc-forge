use std::{
    fmt::Debug,
    ops::{Add, BitXor, Div, Mul, Rem},
};

use crate::error::{CRCResult, Error};

/// A polynomial can either be in normal or reverse representation.
pub enum PolynomialRepr<T> {
    /// Normal polynomial representation (MSB is term of highest degree in polynomial)
    Normal(T),
    /// Reverse polynomial representation (LSB is term of highest degree in polynomial)
    Reverse(T),
}

/// A polynomial with internal representation in reverse order.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Polynomial<T>(T);

impl<T> Polynomial<T>
where
    T: Copy,
{
    pub fn repr(&self) -> T {
        self.0
    }
}

/***************************************
 * Polynomial initialization from repr *
 **************************************/

impl From<PolynomialRepr<u32>> for Polynomial<u32> {
    fn from(repr: PolynomialRepr<u32>) -> Self {
        match repr {
            PolynomialRepr::Normal(val) => Self(reverse_u32(val)),
            PolynomialRepr::Reverse(val) => Self(val),
        }
    }
}

impl From<PolynomialRepr<u64>> for Polynomial<u64> {
    fn from(repr: PolynomialRepr<u64>) -> Self {
        match repr {
            PolynomialRepr::Normal(val) => Self(reverse_u64(val)),
            PolynomialRepr::Reverse(val) => Self(val),
        }
    }
}

impl From<PolynomialRepr<u128>> for Polynomial<u128> {
    fn from(repr: PolynomialRepr<u128>) -> Self {
        match repr {
            PolynomialRepr::Normal(val) => Self(reverse_u128(val)),
            PolynomialRepr::Reverse(val) => Self(val),
        }
    }
}

/*************************************************************
 * Conversion between polynomials and integer representation *
 ************************************************************/

// impl<T> Into<T> for Polynomial<T> {

// }

/*****************************************************
 * Conversion between polynomials of different sizes *
 ****************************************************/

impl TryFrom<Polynomial<u128>> for Polynomial<u64> {
    type Error = Error;
    fn try_from(value: Polynomial<u128>) -> Result<Self, Self::Error> {
        if value.repr() & 0xffffffffffffffff != 0 {
            return Err(Error::OverflowError(None));
        }
        Ok(Polynomial((value.repr() >> 64).try_into()?))
    }
}

impl TryFrom<Polynomial<u128>> for Polynomial<u32> {
    type Error = Error;
    fn try_from(value: Polynomial<u128>) -> Result<Self, Self::Error> {
        if value.repr() & 0xffffffffffffffffffffffff != 0 {
            return Err(Error::OverflowError(None));
        }
        Ok(Polynomial((value.repr() >> 96).try_into()?))
    }
}

impl TryFrom<Polynomial<u64>> for Polynomial<u32> {
    type Error = Error;
    fn try_from(value: Polynomial<u64>) -> Result<Self, Self::Error> {
        if value.repr() & 0xffffffff != 0 {
            return Err(Error::OverflowError(None));
        }
        Ok(Polynomial((value.repr() >> 32).try_into()?))
    }
}

impl From<Polynomial<u64>> for Polynomial<u128> {
    fn from(value: Polynomial<u64>) -> Self {
        Self(u128::from(value.repr()) << 64)
    }
}

impl From<Polynomial<u32>> for Polynomial<u128> {
    fn from(value: Polynomial<u32>) -> Self {
        Self::from(Polynomial::<u64>::from(value))
    }
}

impl From<Polynomial<u32>> for Polynomial<u64> {
    fn from(value: Polynomial<u32>) -> Self {
        Self(u64::from(value.repr()) << 32)
    }
}

/********************************
 * Polynomial addition in F2[X] *
 *******************************/

impl<T> Add<Polynomial<T>> for Polynomial<T>
where
    T: BitXor<T, Output = T>,
    T: Copy,
{
    type Output = Polynomial<T>;
    fn add(self, rhs: Polynomial<T>) -> Self::Output {
        Polynomial(self.repr() ^ rhs.repr())
    }
}

/**************************************
 * Polynomial multiplication in F2[X] *
 *************************************/

impl<T> Mul<T> for Polynomial<u64>
where
    T: Into<Polynomial<u64>>,
{
    type Output = Polynomial<u128>;
    fn mul(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        let mut self_bits = self.repr();
        let mut res_bits = 0u128;
        let rhs_bits = rhs.repr();
        for i in (0..64).rev() {
            if self_bits & 1 == 1 {
                res_bits ^= u128::from(rhs_bits) << (64 - i);
            }
            self_bits >>= 1;
        }
        Polynomial(res_bits)
    }
}

impl Mul<Polynomial<u32>> for Polynomial<u32> {
    type Output = Polynomial<u64>;
    fn mul(self, rhs: Polynomial<u32>) -> Self::Output {
        let self_u64: Polynomial<u64> = self.into();
        let rhs: Polynomial<u64> = rhs.into();
        (self_u64 * rhs).try_into().unwrap()
    }
}

impl Mul<Polynomial<u64>> for Polynomial<u32> {
    type Output = Polynomial<u128>;
    fn mul(self, rhs: Polynomial<u64>) -> Self::Output {
        let self_u64: Polynomial<u64> = self.into();
        (self_u64 * rhs).try_into().unwrap()
    }
}

/********************************************
 * Quotient in polynomial division in F2[X] *
 *******************************************/

impl<T> Div<T> for Polynomial<u128>
where
    T: Into<Polynomial<u128>>,
{
    type Output = Polynomial<u128>;

    fn div(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        let mut res: u128 = 0;
        let mut self_bits = self.repr();
        let rhs_deg = rhs.deg();
        // Number of check steps to perform
        let steps = (128 - rhs_deg) as u32;
        // Remove highest degree term (it is shifted anyway)
        let (rhs_bits, _) = rhs.repr().overflowing_shr(steps);

        for _ in 0..steps {
            res <<= 1;
            let div = self_bits & 1;
            self_bits >>= 1;
            if div == 1 {
                res ^= 1;
                self_bits ^= rhs_bits;
            }
        }

        // Obtained polynomial is in normal representation
        Polynomial::from(PolynomialRepr::Normal(res))
    }
}

impl<T> Div<T> for Polynomial<u64>
where
    T: Into<Polynomial<u128>>,
{
    type Output = Polynomial<u64>;
    fn div(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        let self_u128: Polynomial<u128> = self.into();
        (self_u128 / rhs).try_into().unwrap()
    }
}

impl<T> Div<T> for Polynomial<u32>
where
    T: Into<Polynomial<u128>>,
{
    type Output = Polynomial<u32>;
    fn div(self, rhs: T) -> Self::Output {
        let rhs = rhs.into();
        let self_u128: Polynomial<u128> = self.into();
        (self_u128 / rhs).try_into().unwrap()
    }
}

/*********************************************
 * Remainder in polynomial division in F2[X] *
 ********************************************/

impl Rem<Polynomial<u128>> for Polynomial<u128> {
    type Output = Polynomial<u128>;
    fn rem(self, modulo: Polynomial<u128>) -> Self::Output {
        let mut self_bits: u128 = self.repr();
        let modulo_deg = modulo.deg();
        // Number of check steps to perform
        let steps = (128 - modulo_deg) as u32;
        // Remove the highest degree term (it is shifted anyway)
        let (modulo_bits, _) = modulo.repr().overflowing_shr(steps);

        for _ in 0..steps {
            let div = self_bits & 1;
            self_bits >>= 1;
            if div == 1 {
                self_bits ^= modulo_bits;
            }
        }

        // Shift back number of steps
        // Obtained polynomial is in reverse representation
        Polynomial(self_bits.overflowing_shl(steps).0)
    }
}

impl Rem<Polynomial<u128>> for Polynomial<u64> {
    type Output = Polynomial<u128>;
    fn rem(self, modulo: Polynomial<u128>) -> Self::Output {
        let self_u128: Polynomial<u128> = self.into();
        self_u128 % modulo
    }
}

impl Rem<Polynomial<u128>> for Polynomial<u32> {
    type Output = Polynomial<u128>;
    fn rem(self, modulo: Polynomial<u128>) -> Self::Output {
        let self_u128: Polynomial<u128> = self.into();
        self_u128 % modulo
    }
}

impl Rem<Polynomial<u64>> for Polynomial<u128> {
    type Output = Polynomial<u64>;
    fn rem(self, modulo: Polynomial<u64>) -> Self::Output {
        let modulo: Polynomial<u128> = modulo.into();
        (self % modulo).try_into().unwrap()
    }
}

impl Rem<Polynomial<u64>> for Polynomial<u64> {
    type Output = Polynomial<u64>;
    fn rem(self, modulo: Polynomial<u64>) -> Self::Output {
        let modulo: Polynomial<u128> = modulo.into();
        (self % modulo).try_into().unwrap()
    }
}

impl Rem<Polynomial<u64>> for Polynomial<u32> {
    type Output = Polynomial<u32>;
    fn rem(self, modulo: Polynomial<u64>) -> Self::Output {
        let modulo: Polynomial<u128> = modulo.into();
        (self % modulo).try_into().unwrap()
    }
}

impl Rem<Polynomial<u32>> for Polynomial<u128> {
    type Output = Polynomial<u32>;
    fn rem(self, modulo: Polynomial<u32>) -> Self::Output {
        let modulo: Polynomial<u128> = modulo.into();
        (self % modulo).try_into().unwrap()
    }
}

impl Rem<Polynomial<u32>> for Polynomial<u64> {
    type Output = Polynomial<u32>;
    fn rem(self, modulo: Polynomial<u32>) -> Self::Output {
        let modulo: Polynomial<u128> = modulo.into();
        (self % modulo).try_into().unwrap()
    }
}

impl Rem<Polynomial<u32>> for Polynomial<u32> {
    type Output = Polynomial<u32>;
    fn rem(self, modulo: Polynomial<u32>) -> Self::Output {
        let modulo: Polynomial<u128> = modulo.into();
        (self % modulo).try_into().unwrap()
    }
}

/************************
 * Degree of polynomial *
 ***********************/

impl<T> Polynomial<T>
where
    T: Into<u128>,
    T: Copy,
{
    pub fn deg(self) -> u32 {
        let mut self_bits = self.repr().into();
        let maxdeg = 8 * size_of::<T>() - 1;
        for i in 0..(maxdeg + 1) {
            if self_bits & 1 == 1 {
                return (maxdeg - i) as u32;
            }
            self_bits >>= 1;
        }
        0
    }
}

impl Polynomial<u64> {
    /// Try to compute modular inverse of given polynomial mod `p`.
    pub fn inv_mod(self, p: Polynomial<u64>) -> CRCResult<Polynomial<u64>> {
        let one = Polynomial::from(PolynomialRepr::Normal(1u64));
        let mut a = p;

        // First get remainder by current polynomial to ensure `deg(self) < deg(p)`
        let mut b = self % a;

        // Then initialize sequence
        let mut vn = Polynomial::from(PolynomialRepr::Normal(0u64));
        let mut vn_1 = Polynomial::from(PolynomialRepr::Normal(1u64));

        loop {
            if b.repr() == 0 {
                return Err(Error::NonInvertibleError);
            }

            // Compute euclidian division
            let q = a / b;
            let r = a % b;

            // Compute next term in sequence
            let tmp = vn_1;
            let prod = (vn_1 * q) % p;
            vn_1 = vn + prod;
            vn = tmp;

            // Remainder is 1: end euclide algorithm
            if r == one {
                return Ok(vn_1 % p);
            }

            // Update a and b
            a = b.into();
            b = r;
        }
    }
}

/***************************
 * Debug and display stuff *
 **************************/

impl Debug for Polynomial<u128> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut bits = self.repr();
        let mut terms = Vec::<usize>::new();
        for i in (0..128).rev() {
            let coef = bits & 1;
            if coef == 1 {
                terms.push(i);
            }
            bits >>= 1;
        }

        if terms.len() == 0 {
            write!(f, "0")?;
        } else {
            write!(
                f,
                "{}",
                terms
                    .into_iter()
                    .map(|i| format!("X^{}", i))
                    .collect::<Vec<String>>()
                    .join(" + ")
            )?;
        }

        Ok(())
    }
}

impl Debug for Polynomial<u64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", Polynomial::<u128>::from(self.clone()))
    }
}

impl Debug for Polynomial<u32> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", Polynomial::<u128>::from(self.clone()))
    }
}

/**********************************
 * Bit operation helper functions *
 *********************************/

/// Reverse bit notation in a u32
pub fn reverse_u32(n: u32) -> u32 {
    let mut res = 0u32;
    for i in 0..16 {
        res |= ((n >> (31 - i)) & 1) << i;
        res |= ((n >> i) & 1) << (31 - i);
    }
    res
}

/// Reverse bit notation in a u64
pub fn reverse_u64(n: u64) -> u64 {
    let mut res = 0u64;
    for i in 0..32 {
        res |= ((n >> (63 - i)) & 1) << i;
        res |= ((n >> i) & 1) << (63 - i);
    }
    res
}

/// Reverse bit notation in a u128
pub fn reverse_u128(n: u128) -> u128 {
    let mut res = 0u128;
    for i in 0..64 {
        res |= ((n >> (127 - i)) & 1) << i;
        res |= ((n >> i) & 1) << (127 - i);
    }
    res
}

#[cfg(test)]
mod tests {
    use crate::math::{Polynomial, PolynomialRepr};

    #[test]
    pub fn test_simple_add() {
        let a = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let b = Polynomial::from(PolynomialRepr::Normal(0x12341234u32));
        assert_eq!(a + b, Polynomial::from(PolynomialRepr::Normal(0x16f50f83)));

        let a: Polynomial<u128> = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32)).into();
        let b = Polynomial::from(PolynomialRepr::Normal(0x3429182a00424242u128));
        assert_eq!(
            a + b,
            Polynomial::from(PolynomialRepr::Normal(0x3429182a04835ff5))
        );
    }

    #[test]
    pub fn test_simple_mul() {
        let a = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let b = Polynomial::from(PolynomialRepr::Normal(0x100000000u64));
        assert_eq!(
            a * b,
            Polynomial::from(PolynomialRepr::Normal(0x04c11db700000000u128))
        );

        let a = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let b = Polynomial::from(PolynomialRepr::Normal(0x3429182au32));
        assert_eq!(
            a * b,
            Polynomial::from(PolynomialRepr::Normal(0xc78c9ba470a836))
        );
    }

    #[test]
    pub fn test_simple_div() {
        let a = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let b = Polynomial::from(PolynomialRepr::Normal(0x12341234u32));
        assert_eq!(a / b, Polynomial::from(PolynomialRepr::Normal(0)));

        let a = Polynomial::from(PolynomialRepr::Normal(0x123412341237u64));
        let b = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let c: Polynomial<u32> = (a / b).try_into().unwrap();
        let r = (c * b) + a;
        assert_eq!(c, Polynomial::from(PolynomialRepr::Normal(0x44009)));
        assert!(r.deg() < b.deg());
    }

    #[test]
    pub fn test_simple_rem() {
        let a = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let b = Polynomial::from(PolynomialRepr::Normal(0x12341234u32));
        assert_eq!(a % b, a);

        let a = Polynomial::from(PolynomialRepr::Normal(0x123412341237u64));
        let b = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let r = a % b;
        let q = a / b;
        assert!(r.deg() < a.deg());
        assert_eq!(a, Polynomial::<u64>::try_from(q * b).unwrap() + r.into());
        assert_eq!(a % b, Polynomial::from(PolynomialRepr::Normal(0x14c2238)));
    }

    #[test]
    pub fn test_degree() {
        let a = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        assert_eq!(a.deg(), 26);
        let b = Polynomial::from(PolynomialRepr::Normal(0x12341234u32));
        assert_eq!(b.deg(), 28);
    }

    #[test]
    pub fn test_debug() {
        let a = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        assert_eq!(
            format!("{:?}", a),
            "X^26 + X^23 + X^22 + X^16 + X^12 + X^11 + X^10 + X^8 + X^7 + X^5 + X^4 + X^2 + X^1 + X^0"
        );
    }

    #[test]
    pub fn test_inv_mod() {
        let generator = Polynomial::from(PolynomialRepr::Normal(0x04c11db7u32));
        let xn = Polynomial::from(PolynomialRepr::Normal(0x100000000u64));
        let generator = xn + generator.into();
        let xn_inv = xn.inv_mod(generator).unwrap();
        assert_eq!(xn_inv, Polynomial::from(PolynomialRepr::Normal(0xcbf1acda)));
        assert_eq!(
            (xn * xn_inv) % generator,
            Polynomial::from(PolynomialRepr::Normal(1))
        );
    }
}
