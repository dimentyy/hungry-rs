use std::ptr::NonNull;

use crate::SerializedLen;
use crate::ser::SerializeUnchecked;

impl<T: SerializedLen> SerializedLen for Option<T> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        if let Some(x) = self {
            x.serialized_len()
        } else {
            0
        }
    }
}

impl<T: SerializeUnchecked> SerializeUnchecked for Option<T> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
        if let Some(x) = self {
            // SAFETY: the caller must uphold the safety contract.
            unsafe { x.serialize_unchecked(buf) }
        } else {
            buf
        }
    }
}
