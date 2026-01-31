use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::mtproto::{Msg, NegativeBytesError};
use crate::{common, tl};

use common::infallible;

use tl::de::Buf;

#[derive(Debug)]
pub enum BufMsgError {
    Insufficient(tl::de::EndOfBufferError),
    NegativeBytes(NegativeBytesError),
    IncompleteBody(tl::de::EndOfBufferError),
    NotEnoughForTyp(tl::de::EndOfBufferError),
}

impl fmt::Display for BufMsgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BufMsgError::*;

        f.write_str("`mtproto::BufMsg` deserialization error: ")?;

        match self {
            Insufficient(err) => {
                f.write_str("insufficient length to read the header: ")?;
                err.fmt(f)
            }
            NegativeBytes(err) => err.fmt(f),
            IncompleteBody(err) => {
                f.write_str("incomplete body or length is too large: ")?;
                err.fmt(f)
            }
            NotEnoughForTyp(err) => {
                f.write_str("not enough length to read 4-byte `typ`: ")?;
                err.fmt(f)
            }
        }
    }
}

impl std::error::Error for BufMsgError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use BufMsgError::*;

        Some(match self {
            Insufficient(err) | IncompleteBody(err) | NotEnoughForTyp(err) => err,
            NegativeBytes(err) => err,
        })
    }
}

#[must_use]
#[derive(Clone, Debug)]
pub struct BufMsg<'a> {
    pub msg: Msg,
    pub typ: u32,
    pub buf: Buf<'a>,
}

impl Deref for BufMsg<'_> {
    type Target = Msg;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.msg
    }
}

impl DerefMut for BufMsg<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.msg
    }
}

impl<'a> BufMsg<'a> {
    /// # Errors
    ///
    /// * If the provided [`Buf`] does not have enough capacity.
    /// * If the `bytes` field in a deserialized [`Msg`] is negative.
    pub fn deserialize(buf: &mut Buf<'a>) -> Result<Self, BufMsgError> {
        use BufMsgError::*;

        let header = buf.take_exactly::<16>().map_err(Insufficient)?;

        infallible! {
            let msg_id = i64::from_le_bytes(header[0..8].try_into().unwrap());
            let seq_no = i32::from_le_bytes(header[8..12].try_into().unwrap());
            let bytes = i32::from_le_bytes(header[12..16].try_into().unwrap());
        }

        let msg = Msg { msg_id, seq_no };

        let Ok(bytes) = usize::try_from(bytes) else {
            return Err(NegativeBytes(NegativeBytesError(bytes)));
        };

        let mut buf = Buf::new(buf.take(bytes).map_err(IncompleteBody)?);

        let typ = buf.de_infallible().map_err(NotEnoughForTyp)?;

        Ok(Self { msg, typ, buf })
    }
}
