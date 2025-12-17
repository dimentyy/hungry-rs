use std::{fmt, io};

use crate::transport;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Transport(transport::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("reader error: ")?;

        match self {
            Error::Io(err) => err.fmt(f),
            Error::Transport(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    #[inline]
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<transport::Error> for Error {
    #[inline]
    fn from(value: transport::Error) -> Self {
        Self::Transport(value)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Error::Io(err) => err,
            Error::Transport(err) => err,
        })
    }
}
