use crate::de::{Buf, Deserialize, DeserializeInfallible, Error};
use crate::{BareVec, VECTOR};

pub unsafe fn deserialize_vec<T: Deserialize>(buf: &mut Buf) -> Result<Vec<T>, Error> {
    let len = unsafe { u32::deserialize_infallible(buf.advance_unchecked(4)) } as usize;

    let mut vec = Vec::with_capacity(len);

    for _ in 0..len {
        vec.push(T::deserialize(buf)?);
    }

    Ok(vec)
}

impl<T: Deserialize> Deserialize for Vec<T> {
    #[inline(always)]
    fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        buf.check_len(8)?;

        unsafe {
            let id = u32::deserialize_infallible(buf.advance_unchecked(4));

            if id != VECTOR {
                return Err(Error::UnexpectedConstructor);
            }

            deserialize_vec(buf)
        }
    }
}

impl<T: Deserialize> Deserialize for BareVec<T> {
    #[inline(always)]
    fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        buf.check_len(4)?;

        unsafe { deserialize_vec(buf) }.map(BareVec)
    }
}
