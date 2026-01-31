use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::mtproto::Msg;
use crate::{common, tl};

use common::infallible;

use tl::de::Buf;

#[derive(Debug)]
pub enum BufMsgError {
    EndOfDeBuffer(tl::de::EndOfBufferError),
    NegativeBytes(i32),
}

impl fmt::Display for BufMsgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BufMsgError::*;

        f.write_str("`mtproto::BufMsg` deserialization error: ")?;

        match self {
            EndOfDeBuffer(err) => err.fmt(f),
            NegativeBytes(len) => write!(f, "negative length: {len}"),
        }
    }
}

impl std::error::Error for BufMsgError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use BufMsgError::*;

        match self {
            EndOfDeBuffer(err) => Some(err),
            NegativeBytes(_) => None,
        }
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

    fn deref(&self) -> &Self::Target {
        &self.msg
    }
}

impl DerefMut for BufMsg<'_> {
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

        let header = buf.take_exactly::<16>().map_err(EndOfDeBuffer)?;

        infallible! {
            let msg_id = i64::from_le_bytes(header[0..8].try_into().unwrap());
            let seq_no = i32::from_le_bytes(header[8..12].try_into().unwrap());
            let bytes = i32::from_le_bytes(header[12..16].try_into().unwrap());
        }

        let msg = Msg { msg_id, seq_no };

        let Ok(bytes) = usize::try_from(bytes) else {
            return Err(NegativeBytes(bytes));
        };

        let mut buf = Buf::new(buf.take(bytes).map_err(EndOfDeBuffer)?);

        let typ = buf.de_infallible().map_err(EndOfDeBuffer)?;

        Ok(Self { msg, typ, buf })
    }
}
