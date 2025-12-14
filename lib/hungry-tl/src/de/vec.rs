use crate::de::{Buf, Deserialize, DeserializeHybrid, DeserializeInfallible, Error};
use crate::{BareVec, VECTOR};

pub unsafe fn deserialize_vec<T: Deserialize>(buf: &mut Buf) -> Result<Vec<T>, Error> {
    let len = unsafe { u32::deserialize_infallible(buf.advance_unchecked(4)) } as usize;

    let mut vec = Vec::with_capacity(len);

    for _ in 0..len {
        vec.push(T::deserialize(buf)?);
    }

    Ok(vec)
}

impl<T: Deserialize> DeserializeHybrid for Vec<T> {
    const HYBRID_DESERIALIZATION_UNCHECKED_UNTIL: usize = 4 + 4;

    #[inline(always)]
    unsafe fn deserialize_hybrid(buf: &mut Buf) -> Result<Self, Error> {
        unsafe {
            let id = u32::deserialize_infallible(buf.advance_unchecked(4));

            if id != VECTOR {
                return Err(Error::UnexpectedConstructor);
            }

            deserialize_vec(buf)
        }
    }
}

impl<T: Deserialize> DeserializeHybrid for BareVec<T> {
    const HYBRID_DESERIALIZATION_UNCHECKED_UNTIL: usize = 4;

    #[inline(always)]
    unsafe fn deserialize_hybrid(buf: &mut Buf) -> Result<Self, Error> {
        unsafe { deserialize_vec(buf) }.map(BareVec)
    }
}
