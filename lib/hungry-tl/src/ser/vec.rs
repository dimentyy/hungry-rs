use std::ptr::NonNull;

use crate::ser::SerializeUnchecked;
use crate::{BareVec, ConstSerializedLen, SerializedLen, VECTOR};

pub fn bare_vec_serialized_len<T: SerializedLen>(arr: &[T]) -> usize {
    let mut sum = u32::SERIALIZED_LEN;

    for x in arr {
        sum += x.serialized_len();
    }

    sum
}

pub unsafe fn bare_vec_serialize_unchecked<T: SerializeUnchecked>(
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
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        bare_vec_serialized_len(&self.0)
    }
}

impl<T: SerializeUnchecked> SerializeUnchecked for BareVec<T> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
        unsafe { bare_vec_serialize_unchecked(&self.0, buf) }
    }
}

impl<T: SerializedLen> SerializedLen for Vec<T> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        u32::SERIALIZED_LEN + bare_vec_serialized_len(self)
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
