use std::ptr::NonNull;
use std::mem::transmute;

use crate::ser::SerializeUnchecked;
use crate::{Int128, Int256};

macro_rules! big_int {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl SerializeUnchecked for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
                unsafe {
                    transmute::<_, NonNull<[u8; $len]>>(buf).write_unaligned(*self);

                    buf.add($len)
                }
            }
        }
    )+ };
}

big_int!(
    Int128 => 16,
    Int256 => 32,
);
