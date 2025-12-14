mod big_int;
mod buf;
mod bytes;
mod error;
mod primitives;
mod vec;

use crate::ConstSerializedLen;

pub use buf::Buf;
pub use error::Error;
pub use vec::{deserialize_vec_infallible, deserialize_vec_unchecked};

pub trait Deserialize: Sized {
    fn deserialize(buf: &mut Buf) -> Result<Self, Error>;
}

pub trait DeserializeHybrid: Sized {
    const HYBRID_DESERIALIZATION_UNCHECKED_UNTIL: usize;

    unsafe fn deserialize_hybrid(buf: &mut Buf) -> Result<Self, Error>;
}

pub trait DeserializeUnchecked: ConstSerializedLen + Sized {
    unsafe fn deserialize_unchecked(buf: *const u8) -> Result<Self, Error>;
}

pub trait DeserializeInfallible: ConstSerializedLen + Sized {
    unsafe fn deserialize_infallible(buf: *const u8) -> Self;
}

impl<T: DeserializeHybrid> Deserialize for T {
    #[inline(always)]
    fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        buf.check_len(Self::HYBRID_DESERIALIZATION_UNCHECKED_UNTIL)?;
        unsafe { Self::deserialize_hybrid(buf) }
    }
}

impl<T: DeserializeUnchecked> DeserializeHybrid for T {
    const HYBRID_DESERIALIZATION_UNCHECKED_UNTIL: usize = T::SERIALIZED_LEN;

    #[inline(always)]
    unsafe fn deserialize_hybrid(buf: &mut Buf) -> Result<Self, Error> {
        unsafe { Self::deserialize_unchecked(buf.advance_unchecked(T::SERIALIZED_LEN)) }
    }
}

impl<T: DeserializeInfallible> DeserializeUnchecked for T {
    #[inline(always)]
    unsafe fn deserialize_unchecked(buf: *const u8) -> Result<Self, Error> {
        Ok(unsafe { Self::deserialize_infallible(buf) })
    }
}
