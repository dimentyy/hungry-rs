use std::ptr::NonNull;

use crate::{mtproto, tl};

use tl::ConstSerializedLen;
use tl::de::DeserializeInfallible;
use tl::ser::SerializeUnchecked;

#[must_use]
#[derive(Debug)]
pub struct Msg {
    pub msg_id: mtproto::MsgId,
    pub seq_no: mtproto::SeqNo,
}

impl Msg {
    pub const HEADER_LEN: usize = Self::SERIALIZED_LEN + i32::SERIALIZED_LEN;
}

impl ConstSerializedLen for Msg {
    const SERIALIZED_LEN: usize = mtproto::MsgId::SERIALIZED_LEN + mtproto::SeqNo::SERIALIZED_LEN;
}

impl SerializeUnchecked for Msg {
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            buf = self.msg_id.serialize_unchecked(buf);
            buf = self.seq_no.serialize_unchecked(buf);
            buf
        }
    }
}

impl DeserializeInfallible for Msg {
    unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self {
        unsafe {
            Self {
                msg_id: i64::deserialize_infallible(buf),
                seq_no: i32::deserialize_infallible(buf.add(8)),
            }
        }
    }
}
