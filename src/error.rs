#[derive(Debug)]
pub enum Error {
    OverflowError(Option<std::num::TryFromIntError>),
    NonInvertibleError,
    IOError(std::io::Error),
    EncodingError,
    OutOfBoundsError,
}

pub type CRCResult<T> = Result<T, Error>;

impl From<std::num::TryFromIntError> for Error {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::OverflowError(Some(err))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err)
    }
}
