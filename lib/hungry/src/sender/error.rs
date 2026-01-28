use std::fmt;

use crate::mtproto::{AuthKeyIdError, BufMsgError, MsgKeyCheckError, SessionIdError};
use crate::reader::ReaderError;
use crate::tl;
use crate::writer::WriterError;

#[derive(Debug)]
pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),

    TooSmall { len: usize },
    AuthKeyId(AuthKeyIdError),
    MsgKeyCheck(MsgKeyCheckError),
    SessionId(SessionIdError),

    Deserialization(tl::de::Error),
    NegativeMsgBytes(i32),
    BufRemainingLen { object: Box<tl::Object>, len: usize },
    PaddingTooSmall { len: usize },
    PaddingTooLarge { len: usize },
}

impl From<tl::de::Error> for SenderError {
    #[inline]
    fn from(value: tl::de::Error) -> Self {
        Self::Deserialization(value)
    }
}

impl From<tl::de::EndOfBufferError> for SenderError {
    #[inline]
    fn from(value: tl::de::EndOfBufferError) -> Self {
        Self::Deserialization(value.into())
    }
}

impl From<BufMsgError> for SenderError {
    #[inline]
    fn from(value: BufMsgError) -> Self {
        use BufMsgError::*;

        match value {
            BufferTooSmall(err) | IncompleteBody(err) => Self::Deserialization(err.into()),
            NegativeLength(len) => Self::NegativeMsgBytes(len),
        }
    }
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

            _ => todo!(),
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

            _ => return None,
        })
    }
}
