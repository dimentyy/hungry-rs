use std::fmt;
use std::str::Utf8Error;

#[derive(Clone, Debug)]
pub enum Error {
    EndOfBuffer(EndOfBufferError),
    UnexpectedConstructor(UnexpectedConstructorError),
    InvalidUtf8String(Utf8Error),
}

impl Error {
    #[inline(always)]
    pub const fn end_of_buffer() -> Self {
        Self::EndOfBuffer(EndOfBufferError {})
    }

    #[inline(always)]
    pub const fn unexpected_constructor() -> Self {
        Self::UnexpectedConstructor(UnexpectedConstructorError {})
    }
}

impl From<EndOfBufferError> for Error {
    fn from(value: EndOfBufferError) -> Self {
        Self::EndOfBuffer(value)
    }
}

impl From<UnexpectedConstructorError> for Error {
    fn from(value: UnexpectedConstructorError) -> Self {
        Self::UnexpectedConstructor(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;

        f.write_str("deserialization error: ")?;

        match self {
            EndOfBuffer(err) => err.fmt(f),
            UnexpectedConstructor(err) => err.fmt(f),
            InvalidUtf8String(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use Error::*;

        Some(match self {
            EndOfBuffer(err) => err,
            UnexpectedConstructor(err) => err,
            InvalidUtf8String(err) => err,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndOfBufferError {}

impl fmt::Display for EndOfBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("end of buffer")
    }
}

impl std::error::Error for EndOfBufferError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnexpectedConstructorError {}

impl fmt::Display for UnexpectedConstructorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unexpected constructor")
    }
}

impl std::error::Error for UnexpectedConstructorError {}
