use std::ptr::NonNull;

use crate::mtproto::{BytesMsg, Msg, MsgId, SeqNo};
use crate::tl;

use tl::ser::SerializeUnchecked;
use tl::{ConstSerializedLen, SerializedLen};

#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MsgBytes(pub i32);

#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MsgWith<T> {
    pub msg: Msg,
    pub obj: T,
}

impl BytesMsg {
    #[inline]
    pub const fn bytes(msg_id: MsgId, seq_no: SeqNo, bytes: i32) -> Self {
        Self {
            msg: Msg { msg_id, seq_no },
            obj: MsgBytes(bytes),
        }
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
            buf = self.obj.0.serialize_unchecked(buf);
        }

        buf
    }
}

impl<X: SerializedLen> SerializedLen for MsgWith<X> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        16 + self.obj.serialized_len()
    }
}

impl<X: SerializeUnchecked> SerializeUnchecked for MsgWith<X> {
    /// # Panics
    ///
    /// * If the `obj.serialized_len()` exceeds the `i32::MAX`.
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        // SAFETY: the `serialized_len` is exactly the
        // sum of `serialized_len` of the `obj` and 16;
        // the caller must uphold the safety contract.
        unsafe {
            buf = self.msg.serialize_unchecked(buf);

            buf = i32::try_from(self.obj.serialized_len())
                .unwrap()
                .serialize_unchecked(buf);

            self.obj.serialize_unchecked(buf)
        }
    }
}
