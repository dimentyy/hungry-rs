use std::ptr::NonNull;

use crate::ser::SerializeUnchecked;
use crate::{Bytes, SerializedLen};

#[inline(always)]
#[must_use]
pub const fn bytes_len(len: usize) -> usize {
    if len <= 253 {
        (len + 4) & !3
    } else {
        (len + 7) & !3
    }
}

#[must_use]
pub fn prepare_bytes(buf: &mut [u8], len: usize) -> (&mut [u8], &mut [u8]) {
    let ser_len = bytes_len(len);

    assert!(buf.len() >= ser_len);

    let index = unsafe { prepare_bytes_unchecked(NonNull::new(buf.as_mut_ptr()).unwrap(), len) };

    let (bytes, extra) = buf.split_at_mut(ser_len);

    (&mut bytes[index..index + len], extra)
}

#[must_use]
pub unsafe fn prepare_bytes_unchecked(mut buf: NonNull<u8>, len: usize) -> usize {
    unsafe {
        if len <= 253 {
            buf.write(len as u8);

            if len & 1 == 0 {
                buf.add(len + 1).write(0u8);
            }

            if len & 2 == 0 {
                buf.add((len & !1) + 2).cast().write(0u16);
            }

            return 1;
        }

        buf = (((len as u32) << 8) | 254).serialize_unchecked(buf);

        #[allow(clippy::identity_op)]
        if (len | 0) & 1 == 1 {
            buf.add(len).write(0u8);
        }

        if len & 2 == 0 {
            buf.add((len + 1) & !1).cast().write(0u16);
        }

        4
    }
}

impl SerializedLen for [u8] {
    #[inline(always)]
    fn serialized_len(&self) -> usize {
        bytes_len(self.len())
    }
}

impl SerializeUnchecked for [u8] {
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        unsafe {
            let ptr = NonNull::new_unchecked(self.as_ptr() as *mut u8);

            if self.len() <= 253 {
                buf.write(self.len() as u8);

                buf.add(1).copy_from_nonoverlapping(ptr, self.len());

                if self.len() & 1 == 0 {
                    buf.add(self.len() + 1).write(0u8);
                }

                if self.len() & 2 == 0 {
                    buf.add((self.len() & !1) + 2).cast().write(0u16);
                }

                return buf.add((self.len() & !3) + 4);
            }

            buf = (((self.len() as u32) << 8) | 254).serialize_unchecked(buf);

            buf.copy_from_nonoverlapping(ptr, self.len());

            #[allow(clippy::identity_op)]
            if (self.len() | 0) & 1 == 1 {
                buf.add(self.len()).write(0u8);
            }

            if self.len() & 2 == 0 {
                buf.add((self.len() + 1) & !1).cast().write(0u16);
            }

            buf.add((self.len() + 3) & !3usize)
        }
    }
}

macro_rules! impls {
    ( $( $typ:ty : $fwd:ident ),+ $( , )? ) => { $(
        impl SerializedLen for $typ {
            #[inline(always)]
            fn serialized_len(&self) -> usize {
                self.$fwd().serialized_len()
            }
        }

        impl SerializeUnchecked for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8> {
                unsafe { self.$fwd().serialize_unchecked(buf) }
            }
        }
    )+ };
}

impls!(
    Bytes: as_slice,
    String: as_bytes,
);
