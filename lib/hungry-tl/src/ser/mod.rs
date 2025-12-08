mod big_int;
mod bytes;
mod primitives;
mod vec;

use ::bytes::BytesMut;

pub use bytes::{bytes_len, prepare_bytes};

pub trait Serialize {
    /// Returns the exact number of bytes required to serialize the instance.
    fn serialized_len(&self) -> usize;

    /// Serializes the instance into `buf` without checking its capacity.
    ///
    /// # Safety
    ///
    /// * `buf` must have at least [`serialized_len`] bytes of capacity.
    ///
    /// [`serialized_len`]: Serialize::serialized_len
    unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8;

    #[inline]
    fn serialize_into<I: SerializeInto>(&self, into: &mut I) {
        into.serialize(self)
    }
}

#[inline]
pub fn into<I: SerializeInto, X: Serialize>(into: &mut I, x: &X) {
    into.serialize(x)
}

fn invalid_ret() -> ! {
    panic!()
}

#[inline(always)]
fn check_ret<X: Serialize + ?Sized>(x: &X, buf: *mut u8, len: usize) {
    if unsafe { buf.add(len) != x.serialize_unchecked(buf) } {
        invalid_ret()
    }
}

fn buf_too_small(required: usize, available: usize) -> ! {
    panic!(
        "buffer too small for serialization: {} bytes required, but only {} available",
        required, available
    );
}

#[inline(always)]
fn check_len<X: Serialize + ?Sized>(x: &X, cap: usize) -> usize {
    let len = x.serialized_len();

    if len > cap {
        buf_too_small(len, cap);
    }

    len
}

pub trait SerializeInto {
    fn serialize<X: Serialize + ?Sized>(&mut self, x: &X);
}

impl SerializeInto for [u8] {
    fn serialize<X: Serialize + ?Sized>(&mut self, x: &X) {
        let len = check_len(x, self.len());

        let buf = self.as_mut_ptr();

        check_ret(x, buf, len);
    }
}

impl<const N: usize> SerializeInto for [u8; N] {
    fn serialize<X: Serialize + ?Sized>(&mut self, x: &X) {
        let len = check_len(x, N);

        let buf = self.as_mut_ptr();

        check_ret(x, buf, len);
    }
}

impl SerializeInto for Vec<u8> {
    fn serialize<X: Serialize + ?Sized>(&mut self, x: &X) {
        let len = x.serialized_len();

        let cap = self.capacity() - self.len();

        if len > cap {
            self.reserve(len - cap);
        }

        let buf = self.spare_capacity_mut().as_mut_ptr() as *mut u8;

        check_ret(x, buf, len);

        unsafe { self.set_len(self.len() + len) };
    }
}

impl SerializeInto for BytesMut {
    fn serialize<X: Serialize + ?Sized>(&mut self, x: &X) {
        let len = x.serialized_len();

        let cap = self.capacity() - self.len();

        if len > cap {
            self.reserve(len - cap);
        }

        let buf = self.spare_capacity_mut().as_mut_ptr() as *mut u8;

        check_ret(x, buf, len);

        unsafe { self.set_len(self.len() + len) };
    }
}
