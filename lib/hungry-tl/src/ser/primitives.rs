use std::ptr;

use crate::{BOOL_FALSE, BOOL_TRUE, ser::Serialize};

macro_rules! int {
    ( $( $num:ty ),+ ) => { $(
        impl Serialize for $num {
            #[inline]
            fn serialized_len(&self) -> usize {
                size_of::<Self>()
            }

            #[inline]
            unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
                unsafe {
                    ptr::copy_nonoverlapping(
                        self.to_le_bytes().as_ptr(),
                        buf,
                        size_of::<Self>()
                    );

                    buf.add(size_of::<Self>())
                }
            }
        }
    )+ };
}

int!(u32, i32, i64);

impl Serialize for f64 {
    #[inline]
    fn serialized_len(&self) -> usize {
        8
    }

    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe {
            ptr::copy_nonoverlapping(self.to_le_bytes().as_ptr(), buf, 8);

            buf.add(8)
        }
    }
}

impl Serialize for bool {
    #[inline]
    fn serialized_len(&self) -> usize {
        4
    }

    #[inline]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe { if *self { BOOL_TRUE } else { BOOL_FALSE }.serialize_unchecked(buf) }
    }
}
