use std::{fmt, io};

use crate::transport::TransportError;

#[derive(Debug)]
pub enum ReaderError {
    Io(io::Error),
    Transport(TransportError),
}

impl fmt::Display for ReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ReaderError::*;

        f.write_str("reader error: ")?;

        match self {
            Io(err) => err.fmt(f),
            Transport(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ReaderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use ReaderError::*;

        Some(match self {
            Io(err) => err,
            Transport(err) => err,
        })
    }
}
