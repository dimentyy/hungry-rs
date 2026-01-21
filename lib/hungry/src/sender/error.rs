use std::fmt;

use crate::mtproto::{AuthKeyIdError, MsgKeyCheckError, SessionIdError};
use crate::reader::ReaderError;
use crate::writer::WriterError;

#[derive(Debug)]
pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),

    TooSmall { len: usize },
    AuthKeyId(AuthKeyIdError),
    MsgKeyCheck(MsgKeyCheckError),
    SessionId(SessionIdError),
}

impl fmt::Display for SenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SenderError::*;

        f.write_str("sender error: ")?;

        match self {
            Reader(err) => err.fmt(f),
            Writer(err) => err.fmt(f),

            TooSmall { len } => write!(f, "message too small: found {len} bytes"),
            AuthKeyId(err) => err.fmt(f),
            MsgKeyCheck(err) => err.fmt(f),
            SessionId(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for SenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use SenderError::*;

        Some(match self {
            Reader(err) => err,
            Writer(err) => err,

            TooSmall { .. } => return None,
            AuthKeyId(err) => err,
            MsgKeyCheck(err) => err,
            SessionId(err) => err,
        })
    }
}
