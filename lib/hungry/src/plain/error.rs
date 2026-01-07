use std::fmt;

use crate::reader::ReaderError;
use crate::writer::WriterError;
use crate::{mtproto, tl};

#[must_use]
#[derive(Debug)]
pub enum PlainError {
    Reader(ReaderError),
    Writer(WriterError),
    EncryptedMessage(mtproto::ExternalHeader),
    Deserialization(tl::de::Error),
}

impl From<ReaderError> for PlainError {
    #[inline]
    fn from(value: ReaderError) -> Self {
        Self::Reader(value)
    }
}

impl From<WriterError> for PlainError {
    #[inline]
    fn from(value: WriterError) -> Self {
        Self::Writer(value)
    }
}

impl From<tl::de::Error> for PlainError {
    #[inline]
    fn from(value: tl::de::Error) -> Self {
        Self::Deserialization(value)
    }
}

impl fmt::Display for PlainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PlainError::*;
        
        f.write_str("plain error: ")?;

        match self {
            Reader(err) => err.fmt(f),
            Writer(err) => err.fmt(f),
            EncryptedMessage(_) => todo!(),
            Deserialization(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for PlainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use PlainError::*;

        Some(match self {
            Reader(err) => err,
            Writer(err) => err,
            EncryptedMessage(_) => return None,
            Deserialization(err) => err,
        })
    }
}
