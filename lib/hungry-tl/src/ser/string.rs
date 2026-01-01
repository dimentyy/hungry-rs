use std::ptr::NonNull;

use crate::Bytes;
use crate::ser::SerializeUnchecked;
use crate::{ConstSerializedLen, SerializedLen};

#[must_use]
#[inline(always)]
pub const fn string_len(len: usize) -> usize {
    if len < 0xFE {
        (len + 4) & !3
    } else if len < (1 << 24) {
        (len + 7) & !3
    } else {
        (len + 11) & !3
    }
}

impl<const N: usize> ConstSerializedLen for [u8; N] {
    const SERIALIZED_LEN: usize = string_len(N);
}

impl SerializedLen for [u8] {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        string_len(self.len())
    }
}

#[inline(always)]
unsafe fn serialize_unchecked_small(src: NonNull<u8>, len: usize, buf: NonNull<u8>) -> NonNull<u8> {
    unsafe {
        buf.write(len as u8);

        buf.add(1).copy_from_nonoverlapping(src, len);

        if len & 1 == 0 {
            buf.add(len + 1).write(0);
        }

        if len & 2 == 0 {
            buf.add((len & !1) + 2).cast::<u16>().write(0);
        }

        buf.add((len & !3) + 4)
    }
}

#[inline(always)]
unsafe fn serialize_unchecked_big(
    src: NonNull<u8>,
    len: usize,
    mut buf: NonNull<u8>,
) -> NonNull<u8> {
    unsafe {
        buf = (((len as u32) << 8) | 0xFE).serialize_unchecked(buf);

        buf.copy_from_nonoverlapping(src, len);

        if len & 1 == 1 {
            buf.add(len).write(0);
        }

        if (len + 1) & 2 == 1 {
            buf.add((len + 1) & !1).cast::<u16>().write(0);
        }

        buf.add((len + 3) & !3)
    }
}

#[inline(always)]
unsafe fn serialize_unchecked_large(
    src: NonNull<u8>,
    len: usize,
    mut buf: NonNull<u8>,
) -> NonNull<u8> {
    unsafe {
        buf = (((len as u32) << 8) | 0xFF).serialize_unchecked(buf);
        buf = ((len >> 24) as u32).serialize_unchecked(buf);

        buf.copy_from_nonoverlapping(src, len);

        if len & 1 == 1 {
            buf.add(len).write(0);
        }

        if (len + 1) & 2 == 1 {
            buf.add((len + 1) & !1).cast::<u16>().write(0);
        }

        buf.add((len + 3) & !3)
    }
}

impl SerializeUnchecked for [u8] {
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            let src = NonNull::new_unchecked(self.as_ptr().cast_mut());
            let len = self.len();

            if self.len() < 0xFE {
                serialize_unchecked_small(src, len, buf)
            } else if self.len() < (1 << 24) {
                serialize_unchecked_big(src, len, buf)
            } else if self.len() < (1 << 56) {
                serialize_unchecked_large(src, len, buf)
            } else {
                panic!("string is too large");
            }
        }
    }
}

macro_rules! impls {
    ( $self:ident ; $( $typ:ty => $slice:expr ),+ $( , )? ) => { $(
        impl SerializedLen for $typ {
            #[inline(always)]
            fn serialized_len(&$self) -> usize {
                $slice.serialized_len()
            }
        }

        impl SerializeUnchecked for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&$self, buf: NonNull<u8>) -> NonNull<u8> {
                unsafe { $slice.serialize_unchecked(buf) }
            }
        }
    )+ };
}

impls!(self;
    Vec<u8> => self.as_slice(),
    Bytes => self.0.as_slice(),
    String => self.as_bytes(),
);
