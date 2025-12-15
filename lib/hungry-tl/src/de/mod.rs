mod big_int;
mod buf;
mod bytes;
mod error;
mod primitives;
mod vec;

use std::ptr::NonNull;
use crate::{ConstSerializedLen, SerializedLen};

pub use buf::Buf;
pub use error::{Error, UnexpectedConstructorError, EndOfBufferError};

pub trait Deserialize: SerializedLen + Sized {
    /// # Safety
    ///
    /// * The [`serialized_len`] of the instance _should_ be checked afterward.
    ///
    /// [`serialized_len`]: SerializedLen::serialized_len
    fn deserialize(buf: &mut Buf) -> Result<Self, Error>;
}

pub trait DeserializeUnchecked: ConstSerializedLen + Sized {
    unsafe fn deserialize_unchecked(buf: NonNull<u8>) -> Result<Self, UnexpectedConstructorError>;
}

pub trait DeserializeInfallible: ConstSerializedLen + Sized {
    unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self;
}

impl<T: DeserializeUnchecked> Deserialize for T {
    #[inline(always)]
    fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        Ok(unsafe { Self::deserialize_unchecked(buf.advance(T::SERIALIZED_LEN)?)? })
    }
}

impl<T: DeserializeInfallible> DeserializeUnchecked for T {
    #[inline(always)]
    unsafe fn deserialize_unchecked(buf: NonNull<u8>) -> Result<Self, UnexpectedConstructorError> {
        Ok(unsafe { Self::deserialize_infallible(buf) })
    }
}
