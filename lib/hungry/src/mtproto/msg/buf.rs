use std::fmt;

use crate::mtproto::Msg;
use crate::{common, tl};

use common::infallible;

use tl::de::Buf;

#[derive(Debug)]
pub enum BufMsgError {
    BufferTooSmall(tl::de::EndOfBufferError),
    NegativeLength,
    IncompleteBody(tl::de::EndOfBufferError),
}

impl fmt::Display for BufMsgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BufMsgError::*;

        f.write_str("`mtproto::BufMsg` deserialization error: ")?;

        match self {
            BufferTooSmall(err) => write!(f, "buffer too small: {err} bytes"),
            NegativeLength => f.write_str("negative length"),
            IncompleteBody(err) => write!(f, "incomplete body: {err}"),
        }
    }
}

impl std::error::Error for BufMsgError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use BufMsgError::*;

        Some(match self {
            BufferTooSmall(err) | IncompleteBody(err) => err,
            NegativeLength => return None,
        })
    }
}

#[must_use]
#[derive(Clone)]
pub struct BufMsg<'a> {
    pub msg: Msg,
    pub buf: Buf<'a>,
}

impl<'a> BufMsg<'a> {
    /// # Errors
    ///
    /// * If the provided `buf` does not have enough capacity.
    /// * If the `bytes` field in a deserialized `Msg` is negative.
    pub fn deserialize(buf: &mut Buf<'a>) -> Result<Self, BufMsgError> {
        use BufMsgError::*;

        let header = buf.take_exactly::<16>().map_err(BufferTooSmall)?;

        infallible! {
            let msg_id = i64::from_le_bytes(header[0..8].try_into().unwrap());
            let seq_no = i32::from_le_bytes(header[8..12].try_into().unwrap());
            let bytes = i32::from_le_bytes(header[12..16].try_into().unwrap());
        }

        let msg = Msg { msg_id, seq_no };

        let Ok(bytes) = usize::try_from(bytes) else {
            return Err(NegativeLength);
        };

        let buf = Buf::new(buf.take(bytes).map_err(IncompleteBody)?);

        Ok(Self { msg, buf })
    }
}
