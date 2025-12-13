use std::ptr;

use crate::ser::Serialize;

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

    let index = unsafe { prepare_bytes_unchecked(buf.as_mut_ptr(), len) };

    let (bytes, extra) = buf.split_at_mut(ser_len);

    (&mut bytes[index..index + len], extra)
}

#[must_use]
pub unsafe fn prepare_bytes_unchecked(mut buf: *mut u8, len: usize) -> usize {
    unsafe {
        if len <= 253 {
            *buf = len as u8;

            if len & 1 == 0 {
                *buf.add(len + 1) = 0;
            }

            if len & 2 == 0 {
                *(buf.add((len & !1) + 2) as *mut u16) = 0;
            }

            return 1;
        }

        buf = (((len as u32) << 8) | 254).serialize_unchecked(buf);

        #[allow(clippy::identity_op)]
        if (len | 0) & 1 == 1 {
            *buf.add(len) = 0;
        }

        if len & 2 == 0 {
            *(buf.add((len + 1) & !1) as *mut u16) = 0;
        }

        4
    }
}

impl Serialize for [u8] {
    #[inline]
    fn serialized_len(&self) -> usize {
        bytes_len(self.len())
    }

    unsafe fn serialize_unchecked(&self, mut buf: *mut u8) -> *mut u8 {
        unsafe {
            if self.len() <= 253 {
                *buf = self.len() as u8;

                ptr::copy_nonoverlapping(self.as_ptr(), buf.add(1), self.len());

                if self.len() & 1 == 0 {
                    *buf.add(self.len() + 1) = 0;
                }

                if self.len() & 2 == 0 {
                    *(buf.add((self.len() & !1) + 2) as *mut u16) = 0;
                }

                return buf.add((self.len() & !3) + 4);
            }

            buf = (((self.len() as u32) << 8) | 254).serialize_unchecked(buf);

            ptr::copy_nonoverlapping(self.as_ptr(), buf, self.len());

            #[allow(clippy::identity_op)]
            if (self.len() | 0) & 1 == 1 {
                *buf.add(self.len()) = 0;
            }

            if self.len() & 2 == 0 {
                *(buf.add((self.len() + 1) & !1) as *mut u16) = 0;
            }

            buf.add((self.len() + 3) & !3usize)
        }
    }
}

impl Serialize for Vec<u8> {
    #[inline]
    fn serialized_len(&self) -> usize {
        self.as_slice().serialized_len()
    }

    #[inline]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe { self.as_slice().serialize_unchecked(buf) }
    }
}

impl Serialize for String {
    #[inline]
    fn serialized_len(&self) -> usize {
        self.as_bytes().serialized_len()
    }

    #[inline]
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
        unsafe { self.as_bytes().serialize_unchecked(buf) }
    }
}
