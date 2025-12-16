use std::ptr::NonNull;

use crate::ser::SerializeUnchecked;
use crate::{Int128, Int256};

macro_rules! big_int {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl SerializeUnchecked for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
                unsafe {
                    let ptr = NonNull::new_unchecked(self.as_ptr() as *mut u8);

                    buf.copy_from_nonoverlapping(ptr, $len);

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
