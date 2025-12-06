use crate::{BareVec, VECTOR, ser::Serialize};

#[inline]
fn bare_vec_serialized_len<T: Serialize>(vec: &[T]) -> usize {
    vec.iter().map(|x| x.serialized_len()).sum::<usize>()
}

#[inline]
unsafe fn bare_vec_serialize_unchecked<T: Serialize>(
    vec: &Vec<T>,
    mut buf: *mut u8,
) -> *mut u8 {
    unsafe {
        buf = (vec.len() as i32).serialize_unchecked(buf);
        for x in vec {
            buf = x.serialize_unchecked(buf)
        }
        buf
    }
}

impl<T: Serialize> Serialize for BareVec<T> {
    #[inline]
    fn serialized_len(&self) -> usize {
        bare_vec_serialized_len(&self.0)
    }

    #[inline]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe { bare_vec_serialize_unchecked(&self.0, buf) }
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    #[inline]
    fn serialized_len(&self) -> usize {
        4 + bare_vec_serialized_len(self)
    }

    #[inline]
    unsafe fn serialize_unchecked(&self, mut buf: *mut u8) -> *mut u8 {
        unsafe {
            buf = VECTOR.serialize_unchecked(buf);
            bare_vec_serialize_unchecked(self, buf)
        }
    }
}
