use std::ptr;

use crate::{Int128, Int256, ser::Serialize};

macro_rules! big_int {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl Serialize for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
                unsafe {
                    ptr::copy_nonoverlapping(
                        self.as_ptr(),
                        buf,
                        $len,
                    );

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
