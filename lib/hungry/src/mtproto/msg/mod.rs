mod buf;

use std::fmt;
use std::ptr::NonNull;

use crate::{mtproto, tl};

use tl::ConstSerializedLen;
use tl::de::DeserializeInfallible;
use tl::ser::SerializeUnchecked;

pub use buf::{BufMsg, BufMsgError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NegativeBytesError(pub i32);

impl fmt::Display for NegativeBytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "negative value in the `bytes` field: {}", self.0)
    }
}

impl std::error::Error for NegativeBytesError {}

#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Msg {
    pub msg_id: mtproto::MsgId,
    pub seq_no: mtproto::SeqNo,
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
