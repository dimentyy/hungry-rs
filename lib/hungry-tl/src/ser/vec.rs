use std::ptr::NonNull;

use crate::ser::SerializeUnchecked;
use crate::{BareVec, ConstSerializedLen, SerializedLen, VECTOR};

unsafe fn bare_vec_serialize_unchecked<T: SerializeUnchecked>(
    arr: &[T],
    mut buf: NonNull<u8>,
) -> NonNull<u8> {
    unsafe {
        buf = (arr.len() as u32).serialize_unchecked(buf);

        for x in arr {
            buf = x.serialize_unchecked(buf)
        }

        buf
    }
}

impl<T: SerializedLen> SerializedLen for BareVec<T> {
    fn serialized_len(&self) -> usize {
        let mut sum = u32::SERIALIZED_LEN;

        for x in self.iter() {
            sum += x.serialized_len();
        }

        sum
    }
}

impl<T: SerializeUnchecked> SerializeUnchecked for BareVec<T> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
        unsafe { bare_vec_serialize_unchecked(self.0.as_ref(), buf) }
    }
}

impl<T: SerializedLen> SerializedLen for Vec<T> {
    fn serialized_len(&self) -> usize {
        let mut sum = const { u32::SERIALIZED_LEN + u32::SERIALIZED_LEN };

        for x in self.iter() {
            sum += x.serialized_len();
        }

        sum
    }
}

impl<T: SerializeUnchecked> SerializeUnchecked for Vec<T> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            buf = VECTOR.serialize_unchecked(buf);
            bare_vec_serialize_unchecked(self, buf)
        }
    }
}
