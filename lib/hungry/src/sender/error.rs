use std::fmt;

use crate::mtproto::{BufMsgError, MsgIdError, SeqNoError, SessionIdError};
use crate::reader::{EncryptedMessageError, ReaderError};
use crate::tl;
use crate::writer::WriterError;

#[derive(Debug)]
pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),

    Message(EncryptedMessageError),
    Session(SessionIdError),

    MsgId(MsgIdError),
    SeqNo(SeqNoError),

    PaddingLength(usize),
    EndOfDeBuffer(tl::de::EndOfBufferError),
    NegativeBytes(i32),
}

impl From<MsgIdError> for SenderError {
    #[inline]
    fn from(value: MsgIdError) -> Self {
        Self::MsgId(value)
    }
}

impl From<SeqNoError> for SenderError {
    #[inline]
    fn from(value: SeqNoError) -> Self {
        Self::SeqNo(value)
    }
}

impl From<BufMsgError> for SenderError {
    #[inline]
    fn from(value: BufMsgError) -> Self {
        use SenderError::*;

        match value {
            BufMsgError::EndOfDeBuffer(err) => EndOfDeBuffer(err),
            BufMsgError::NegativeBytes(bytes) => NegativeBytes(bytes),
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

            Message(err) => err.fmt(f),
            Session(err) => err.fmt(f),

            MsgId(err) => err.fmt(f),
            SeqNo(err) => err.fmt(f),

            PaddingLength(len) => todo!(),
            EndOfDeBuffer(err) => err.fmt(f),
            NegativeBytes(bytes) => todo!(),
        }
    }
}

impl std::error::Error for SenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use SenderError::*;

        Some(match self {
            Reader(err) => err,
            Writer(err) => err,

            Message(err) => err,
            Session(err) => err,

            MsgId(err) => err,
            SeqNo(err) => err,

            PaddingLength(_) => return None,
            EndOfDeBuffer(err) => err,
            NegativeBytes(_) => return None,
        })
    }
}
