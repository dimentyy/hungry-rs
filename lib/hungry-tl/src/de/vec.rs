use crate::de::{Buf, Deserialize, DeserializeInfallible, DeserializeUnchecked, Error};
use crate::{BareVec, VECTOR};

pub unsafe fn deserialize_vec<T: Deserialize>(buf: &mut Buf) -> Result<Vec<T>, Error> {
    let len = unsafe { u32::deserialize_infallible(buf.advance_unchecked(4)) } as usize;

    let mut vec = Vec::with_capacity(len);

    for _ in 0..len {
        vec.push(T::deserialize_checked(buf)?);
    }

    Ok(vec)
}

impl<T: Deserialize> Deserialize for Vec<T> {
    const MINIMUM_SERIALIZED_LEN: usize = 4 + 4;

    unsafe fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        unsafe {
            let id = u32::deserialize_infallible(buf.advance_unchecked(4));

            if id != VECTOR {
                return Err(Error::UnexpectedConstructor { id });
            }

            deserialize_vec(buf)
        }
    }
}

impl<T: Deserialize> Deserialize for BareVec<T> {
    const MINIMUM_SERIALIZED_LEN: usize = 4;

    unsafe fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        unsafe { deserialize_vec(buf) }.map(BareVec)
    }
}

pub unsafe fn deserialize_vec_unchecked<T: DeserializeUnchecked>(
    buf: &mut Buf,
) -> Result<Vec<T>, Error> {
    let len = unsafe { u32::deserialize_infallible(buf.advance_unchecked(4)) } as usize;

    buf.check_len(T::SERIALIZED_LEN * len)?;

    let mut vec = Vec::with_capacity(len);

    for _ in 0..len {
        vec.push(unsafe { T::deserialize_unchecked(buf.advance_unchecked(T::SERIALIZED_LEN)) }?);
    }

    Ok(vec)
}

pub unsafe fn deserialize_vec_infallible<T: DeserializeInfallible>(
    buf: &mut Buf,
) -> Result<Vec<T>, Error> {
    let len = unsafe { u32::deserialize_infallible(buf.advance_unchecked(4)) } as usize;

    buf.check_len(T::SERIALIZED_LEN * len)?;

    let mut vec = Vec::with_capacity(len);

    for _ in 0..len {
        vec.push(unsafe { T::deserialize_infallible(buf.advance_unchecked(T::SERIALIZED_LEN)) });
    }

    Ok(vec)
}
