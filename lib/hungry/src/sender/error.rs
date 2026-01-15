use std::fmt;

use crate::mtproto::{MsgKeyCheckError, AuthKeyIdError};
use crate::reader::ReaderError;
use crate::writer::WriterError;

#[derive(Debug)]
pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),

    AuthKeyId(AuthKeyIdError),
    MsgKeyCheck(MsgKeyCheckError),

    Todo(&'static str),
}

impl fmt::Display for SenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SenderError::*;

        f.write_str("sender error: ")?;

        match self {
            Reader(err) => err.fmt(f),
            Writer(err) => err.fmt(f),

            AuthKeyId(err) => err.fmt(f),
            MsgKeyCheck(err) => err.fmt(f),

            Todo(todo) => f.write_str(todo),
        }
    }
}

impl std::error::Error for SenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use SenderError::*;

        Some(match self {
            Reader(err) => err,
            Writer(err) => err,

            AuthKeyId(err) => err,
            MsgKeyCheck(err) => err,

            Todo(_) => return None,
        })
    }
}
