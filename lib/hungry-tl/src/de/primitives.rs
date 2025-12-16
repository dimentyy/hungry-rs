use std::ptr::NonNull;

use crate::de::{DeserializeInfallible, DeserializeUnchecked, UnexpectedConstructorError};
use crate::{FALSE, TRUE};

macro_rules! impls {
    ( $buf:ident; $( $typ:ty: $val:expr ),+ $( , )? ) => { $(
        impl DeserializeInfallible for $typ {
            #[inline(always)]
            unsafe fn deserialize_infallible($buf: NonNull<u8>) -> Self {
                unsafe { $val }
            }
        }
    )+ };
}

impls!(buf;
    u32: Self::from_le(buf.cast().read()),
    i32: Self::from_le(buf.cast().read()),
    i64: Self::from_le(buf.cast().read_unaligned()),
    f64: Self::from_bits(i64::deserialize_infallible(buf) as u64)
);

impl DeserializeUnchecked for bool {
    #[inline(always)]
    unsafe fn deserialize_unchecked(buf: NonNull<u8>) -> Result<Self, UnexpectedConstructorError> {
        match unsafe { u32::deserialize_infallible(buf) } {
            TRUE => Ok(true),
            FALSE => Ok(false),
            _ => Err(UnexpectedConstructorError {}),
        }
    }
}
