use std::mem::transmute;
use std::ptr::NonNull;

use crate::de::DeserializeInfallible;
use crate::{Int128, Int256};

macro_rules! big_int {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl DeserializeInfallible for $typ {
            #[inline(always)]
            unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self {
                unsafe { transmute::<_, NonNull<[u8; $len]>>(buf).read_unaligned() }
            }
        }
    )+ };
}

big_int!(
    Int128 => 16,
    Int256 => 32,
);
