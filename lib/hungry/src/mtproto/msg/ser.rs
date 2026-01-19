use std::ptr::NonNull;

use crate::{mtproto, tl};

use tl::SerializedLen;
use tl::ser::SerializeUnchecked;

pub struct MsgSer<'a, X> {
    pub msg: mtproto::Msg,
    pub x: &'a X,
}

impl<'a, X> MsgSer<'a, X> {
    #[inline]
    pub fn new(msg: mtproto::Msg, x: &'a X) -> Self {
        Self { msg, x }
    }
}

impl<'a, X: SerializedLen> SerializedLen for MsgSer<'a, X> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        16 + self.x.serialized_len()
    }
}

impl<'a, X: SerializeUnchecked> SerializeUnchecked for MsgSer<'a, X> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            buf = self.msg.serialize_unchecked(buf);

            buf = i32::try_from(self.x.serialized_len())
                .unwrap()
                .serialize_unchecked(buf);

            self.x.serialize_unchecked(buf)
        }
    }
}
