use std::mem;

use crate::{FALSE, TRUE, ser::Serialize};

impl Serialize for u32 {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe {
            *(buf as *mut Self) = self.to_le();

            buf.add(4)
        }
    }
}

impl Serialize for i32 {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe {
            *(buf as *mut Self) = self.to_le();

            buf.add(4)
        }
    }
}

impl Serialize for i64 {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe {
            (buf as *mut Self).write_unaligned(self.to_le());

            buf.add(8)
        }
    }
}

impl Serialize for f64 {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe { mem::transmute::<&f64, &i64>(self).serialize_unchecked(buf) }
    }
}

impl Serialize for bool {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe { if *self { TRUE } else { FALSE }.serialize_unchecked(buf) }
    }
}
