mod big_int;
mod buf;
mod bytes;
mod error;
mod primitives;
mod vec;

use crate::SerializedLen;

pub use buf::Buf;
pub use error::Error;
pub use vec::{deserialize_vec_infallible, deserialize_vec_unchecked};

pub fn checked<T: Deserialize>(buf: &[u8]) -> Result<T, Error> {
    T::deserialize_checked(&mut Buf::new(buf))
}

pub trait Deserialize: Sized {
    const MINIMUM_SERIALIZED_LEN: usize;

    #[inline]
    fn deserialize_checked(buf: &mut Buf) -> Result<Self, Error> {
        buf.check_len(Self::MINIMUM_SERIALIZED_LEN)?;
        unsafe { Self::deserialize(buf) }
    }

    unsafe fn deserialize(buf: &mut Buf) -> Result<Self, Error>;
}

pub trait DeserializeUnchecked: SerializedLen + Sized {
    unsafe fn deserialize_unchecked(buf: *const u8) -> Result<Self, Error>;
}

pub trait DeserializeInfallible: SerializedLen + Sized {
    unsafe fn deserialize_infallible(buf: *const u8) -> Self;
}

impl<T: DeserializeUnchecked> Deserialize for T {
    const MINIMUM_SERIALIZED_LEN: usize = T::SERIALIZED_LEN;

    #[inline]
    unsafe fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        unsafe { Self::deserialize_unchecked(buf.advance_unchecked(T::SERIALIZED_LEN)) }
    }
}

impl<T: DeserializeInfallible> DeserializeUnchecked for T {
    #[inline]
    unsafe fn deserialize_unchecked(buf: *const u8) -> Result<Self, Error> {
        Ok(unsafe { Self::deserialize_infallible(buf) })
    }
}
