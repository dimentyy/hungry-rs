use std::ptr::NonNull;

use crate::ser::SerializeUnchecked;
use crate::{FALSE, TRUE};

macro_rules! impls {
    ( $self:ident, $buf:ident; $( $typ:ty => $val:expr ),+ $( , )? ) => { $(
        impl SerializeUnchecked for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&$self, $buf: NonNull<u8>) -> NonNull<u8> {
                unsafe {
                    $val;

                    $buf.add(size_of::<Self>())
                }
            }
        }
    )+ };
}

impls!(self, buf;
    u32 => buf.cast().write(self.to_le()),
    i32 => buf.cast().write(self.to_le()),
    i64 => buf.cast().write_unaligned(self.to_le()),
    f64 => buf.cast().write_unaligned(self.to_bits().to_le()),
);

impl SerializeUnchecked for bool {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
        unsafe { if *self { TRUE } else { FALSE }.serialize_unchecked(buf) }
    }
}
