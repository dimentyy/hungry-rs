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

    BufMsg(BufMsgError),
    PaddingLength(usize),
    Deserialization(tl::de::Error),
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
        Self::BufMsg(value)
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

            BufMsg(err) => err.fmt(f),
            PaddingLength(_) => todo!(),
            Deserialization(err) => err.fmt(f),
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

            BufMsg(err) => err,
            PaddingLength(_) => return None,
            Deserialization(err) => err,
        })
    }
}
