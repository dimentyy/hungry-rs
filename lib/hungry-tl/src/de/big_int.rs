use std::ptr::NonNull;

use crate::de::DeserializeInfallible;
use crate::{Int128, Int256};

macro_rules! impls {
    ( $( $typ:ident => $len:expr ),+ $(,)? ) => { $(
        impl DeserializeInfallible for $typ {
            #[inline(always)]
            unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self {
                unsafe { $typ(buf.cast::<[u8; $len]>().read_unaligned()) }
            }
        }
    )+ };
}

impls!(
    Int128 => 16,
    Int256 => 32,
);
