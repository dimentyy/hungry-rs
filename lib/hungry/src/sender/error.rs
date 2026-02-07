use std::fmt;

use crate::mtproto::{BufMsgError, MsgIdError, PaddingError, SeqNoError, SessionIdError};
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

    Msg(BufMsgError),
    Padding(PaddingError),
    Deserialization(tl::de::Error),
    Inflate(zlib_rs::InflateError),

    DoubleGzipPacked,
    DoubleMsgContainer,
}

impl From<tl::de::Error> for SenderError {
    #[inline]
    fn from(value: tl::de::Error) -> Self {
        Self::Deserialization(value)
    }
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
        Self::Msg(value)
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

            Msg(err) => err.fmt(f),
            Padding(err) => err.fmt(f),
            Deserialization(err) => err.fmt(f),
            Inflate(err) => f.write_str(err.as_str()),

            DoubleGzipPacked => todo!(),
            DoubleMsgContainer => todo!(),
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

            Msg(err) => err,
            Padding(err) => err,
            Deserialization(err) => err,
            Inflate(_) => todo!(),

            DoubleGzipPacked => todo!(),
            DoubleMsgContainer => todo!(),
        })
    }
}
