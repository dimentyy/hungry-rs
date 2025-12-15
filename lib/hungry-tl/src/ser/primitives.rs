use std::mem::transmute;
use std::ptr::NonNull;

use crate::ser::SerializeUnchecked;
use crate::{FALSE, TRUE};

macro_rules! impls {
    ( $self:ident, $buf:ident; $( $typ:ty : $add:expr => $ser:expr ),+ $( , )? ) => { $(
        impl SerializeUnchecked for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&$self, $buf: NonNull<u8>) -> NonNull<u8> {
                unsafe {
                    $ser;

                    $buf.add($add)
                }
            }
        }
    )+ };
}

impls!(self, buf;
    u32: 4 => transmute::<_, NonNull<_>>(buf).write(self.to_le()),
    i32: 4 => transmute::<_, NonNull<_>>(buf).write(self.to_le()),
    i64: 8 => transmute::<_, NonNull<_>>(buf).write_unaligned(self.to_le()),
    f64: 8 => transmute::<_, NonNull<_>>(buf).write_unaligned(self.to_bits().to_le()),
);

impl SerializeUnchecked for bool {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
        unsafe { if *self { TRUE } else { FALSE }.serialize_unchecked(buf) }
    }
}
