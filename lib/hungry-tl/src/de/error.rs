use std::fmt;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

#[derive(Clone, Debug)]
pub enum Error {
    UnexpectedConstructor { id: u32 },
    UnexpectedEndOfBuffer,
    InvalidUtf8String(Utf8Error),
}

impl From<FromUtf8Error> for Error {
    #[inline]
    fn from(value: FromUtf8Error) -> Self {
        Self::InvalidUtf8String(value.utf8_error())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("deserialization error: ")?;

        match self {
            Error::UnexpectedConstructor { id } => write!(f, "unexpected constructor {id:08x}"),
            Error::UnexpectedEndOfBuffer => write!(f, "unexpected end of buffer"),
            Error::InvalidUtf8String(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {}
