use std::fmt;

use crate::mtproto::{PlainMessage, Session};
use crate::reader::ReaderError;
use crate::writer::WriterError;

#[derive(Debug)]
pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),

    PlainMessage(PlainMessage),
    UnexpectedAuthKeyId(i64),
    UnexpectedSessionId(Session),
}

impl From<ReaderError> for SenderError {
    #[inline]
    fn from(value: ReaderError) -> Self {
        Self::Reader(value)
    }
}

impl From<WriterError> for SenderError {
    #[inline]
    fn from(value: WriterError) -> Self {
        Self::Writer(value)
    }
}

impl fmt::Display for SenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SenderError::*;

        f.write_str("sender error: ")?;

        match self {
            Reader(err) => err.fmt(f),
            Writer(err) => err.fmt(f),
            PlainMessage(_) => write!(f, "received unexpected plain message"),
            UnexpectedAuthKeyId(err) => write!(f, "unexpected auth key id: {err:#010x}"),
            UnexpectedSessionId(err) => write!(f, "unexpected session id: {err:#010x}"),
        }
    }
}

impl std::error::Error for SenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use SenderError::*;

        match self {
            Writer(err) => Some(err),
            Reader(err) => Some(err),
            _ => None,
        }
    }
}
