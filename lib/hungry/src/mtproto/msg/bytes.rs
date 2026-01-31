use std::ops::Deref;
use std::ptr::NonNull;

use crate::mtproto::Msg;
use crate::tl;

use tl::ConstSerializedLen;
use tl::de::DeserializeInfallible;
use tl::ser::SerializeUnchecked;

#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BytesMsg {
    pub msg: Msg,
    pub bytes: i32,
}

impl Deref for BytesMsg {
    type Target = Msg;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.msg
    }
}

impl ConstSerializedLen for BytesMsg {
    const SERIALIZED_LEN: usize = Msg::SERIALIZED_LEN + i32::SERIALIZED_LEN;
}

impl SerializeUnchecked for BytesMsg {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        // SAFETY: the `SERIALIZED_LEN` is exactly 16;
        // the caller must uphold the safety contract.
        unsafe {
            buf = self.msg.serialize_unchecked(buf);
            buf = self.bytes.serialize_unchecked(buf);
        }

        buf
    }
}

impl DeserializeInfallible for BytesMsg {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self {
        // SAFETY: the `SERIALIZED_LEN` is exactly 16;
        // the caller must uphold the safety contract.
        unsafe {
            Self {
                msg: Msg::deserialize_infallible(buf),
                bytes: i32::deserialize_infallible(buf.add(12)),
            }
        }
    }
}
