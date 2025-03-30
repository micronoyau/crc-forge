use std::{fmt::Debug, num::TryFromIntError};

#[derive(Debug)]
pub enum Error {
    OverflowError(Option<TryFromIntError>),
    NonInvertibleError,
}

pub type CRCResult<T> = Result<T, Error>;

impl From<TryFromIntError> for Error {
    fn from(err: TryFromIntError) -> Self {
        Self::OverflowError(Some(err))
    }
}
