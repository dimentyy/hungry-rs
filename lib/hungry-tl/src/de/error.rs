use std::fmt;

#[derive(Clone, Debug)]
pub enum Error {
    UnexpectedConstructor,
    UnexpectedEndOfBuffer,
    InvalidUtf8String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("deserialization error: ")?;

        match self {
            Error::UnexpectedConstructor => write!(f, "unexpected constructor"),
            Error::UnexpectedEndOfBuffer => write!(f, "unexpected end of buffer"),
            Error::InvalidUtf8String => write!(f, "invalid utf-8 string"),
        }
    }
}

impl std::error::Error for Error {}
