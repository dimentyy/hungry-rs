use crate::{BareVec, ConstSerializedLen, SerializedLen, VECTOR, ser::Serialize};

pub fn bare_vec_serialized_len<T: SerializedLen>(vec: &[T]) -> usize {
    u32::SERIALIZED_LEN + vec.iter().map(|x| x.serialized_len()).sum::<usize>()
}

pub unsafe fn serialize_bare_vec_unchecked<T: Serialize>(vec: &[T], mut buf: *mut u8) -> *mut u8 {
    unsafe {
        buf = (vec.len() as u32).serialize_unchecked(buf);
        for x in vec {
            buf = x.serialize_unchecked(buf)
        }
        buf
    }
}

impl<T: Serialize> SerializedLen for BareVec<T> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        bare_vec_serialized_len(&self.0)
    }
}

impl<T: Serialize> Serialize for BareVec<T> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe { serialize_bare_vec_unchecked(&self.0, buf) }
    }
}

impl<T: Serialize> SerializedLen for Vec<T> {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        u32::SERIALIZED_LEN + bare_vec_serialized_len(self)
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: *mut u8) -> *mut u8 {
        unsafe {
            buf = VECTOR.serialize_unchecked(buf);
            serialize_bare_vec_unchecked(self, buf)
        }
    }
}
