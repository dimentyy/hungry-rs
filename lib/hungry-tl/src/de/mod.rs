mod big_int;
mod buf;
mod error;
mod primitives;
mod string;
mod vec;

use std::ptr::NonNull;

use crate::{ConstSerializedLen, SerializedLen};

pub use buf::Buf;
pub use error::{EndOfBufferError, Error, UnexpectedConstructorError};

pub trait Deserialize: SerializedLen {
    /// # Safety
    ///
    /// * The [`serialized_len`] of the instance _should_ be checked.
    ///
    /// [`serialized_len`]: SerializedLen::serialized_len
    fn deserialize(buf: &mut Buf) -> Result<Self, Error>
    where
        Self: Sized;
}

pub trait DeserializeUnchecked: ConstSerializedLen {
    /// # Safety
    ///
    /// * The `buf` must be valid for at least [`SERIALIZED_LEN`] bytes.
    ///
    /// # Errors
    ///
    /// * When deserialized [`CONSTRUCTOR_ID`] does not match any variants.
    ///
    /// [`SERIALIZED_LEN`]: ConstSerializedLen::SERIALIZED_LEN
    /// [`CONSTRUCTOR_ID`]: crate::Identifiable::CONSTRUCTOR_ID
    unsafe fn deserialize_unchecked(buf: NonNull<u8>) -> Result<Self, UnexpectedConstructorError>
    where
        Self: Sized;
}

pub trait DeserializeInfallible: ConstSerializedLen {
    /// # Safety
    ///
    /// * The `buf` must be valid for at least [`SERIALIZED_LEN`] bytes.
    ///
    /// [`SERIALIZED_LEN`]: ConstSerializedLen::SERIALIZED_LEN
    unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self
    where
        Self: Sized;
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
