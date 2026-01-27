mod buf;
mod with;

use std::fmt;
use std::ptr::NonNull;

use crate::{mtproto, tl};

use tl::ConstSerializedLen;
use tl::de::DeserializeInfallible;
use tl::ser::SerializeUnchecked;

pub use buf::{BufMsg, BufMsgError};
pub use with::{MsgBytes, MsgWith};

/// Type alias representing the `message` constructor header without the `body`.
pub type BytesMsg = MsgWith<MsgBytes>;

#[derive(Debug, Eq, PartialEq)]
pub enum MsgError {
    MsgId(mtproto::MsgIdError),
    SeqNo(mtproto::SeqNoError),
}

impl fmt::Display for MsgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MsgError::*;

        match self {
            MsgId(err) => err.fmt(f),
            SeqNo(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for MsgError {}

#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Msg {
    pub msg_id: mtproto::MsgId,
    pub seq_no: mtproto::SeqNo,
}

impl Msg {
    pub fn validate(
        &self,
        msg_ids: &mut mtproto::ServerMsgIds,
        seq_nos: &mut mtproto::SeqNos,
        unix_time: std::time::SystemTime,
        content_related: bool,
    ) -> Result<mtproto::MsgIdModulus, MsgError> {
        let modulus = msg_ids
            .validate(self.msg_id, unix_time)
            .map_err(MsgError::MsgId)?;

        seq_nos
            .validate(self.seq_no, content_related)
            .map_err(MsgError::SeqNo)?;

        Ok(modulus)
    }
}

impl ConstSerializedLen for Msg {
    const SERIALIZED_LEN: usize = mtproto::MsgId::SERIALIZED_LEN + mtproto::SeqNo::SERIALIZED_LEN;
}

impl SerializeUnchecked for Msg {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        // SAFETY: the `SERIALIZED_LEN` is exactly 12;
        // the caller must uphold the safety contract.
        unsafe {
            buf = self.msg_id.serialize_unchecked(buf);
            buf = self.seq_no.serialize_unchecked(buf);
        }

        buf
    }
}

impl DeserializeInfallible for Msg {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self {
        // SAFETY: the `SERIALIZED_LEN` is exactly 12;
        // the caller must uphold the safety contract.
        unsafe {
            Self {
                msg_id: i64::deserialize_infallible(buf),
                seq_no: i32::deserialize_infallible(buf.add(8)),
            }
        }
    }
}
