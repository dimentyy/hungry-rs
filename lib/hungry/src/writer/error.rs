use std::{fmt, io};

#[derive(Debug)]
pub enum WriterError {
    Io(io::Error),
}

impl fmt::Display for WriterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use WriterError::*;

        f.write_str("reader error: ")?;

        match self {
            Io(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for WriterError {
    #[inline]
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::error::Error for WriterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use WriterError::*;

        Some(match self {
            Io(err) => err,
        })
    }
}
