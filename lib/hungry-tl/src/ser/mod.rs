mod big_int;
mod bytes;
mod primitives;
mod vec;

use std::ptr::NonNull;

use crate::SerializedLen;

pub use bytes::{bytes_len, prepare_bytes};
pub use vec::{bare_vec_serialize_unchecked, bare_vec_serialized_len};

pub trait SerializeUnchecked: SerializedLen {
    /// Serializes the instance into `buf` without checking its capacity.
    ///
    /// # Safety
    ///
    /// * `buf` must have at least [`serialized_len`] bytes of capacity.
    /// * `buf` must be properly aligned for 4-byte (32-bit) writes.
    ///
    /// [`serialized_len`]: SerializedLen::serialized_len
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8>;
}

fn invalid_ret(
    type_name: &str,
    buf: NonNull<u8>,
    len: usize,
    end: NonNull<u8>,
    ret: NonNull<u8>,
) -> ! {
    let off = unsafe { ret.offset_from(end) };

    panic!(
        "`Serialize` implementation for `{type_name}` is invalid: \
        expected `serialize_unchecked` to return {end:?} \
        ({buf:?} + {len:#x}), got {ret:?} off by {off}",
    );
}

#[inline(always)]
fn check_ret<X: SerializeUnchecked + ?Sized>(x: &X, buf: NonNull<u8>, len: usize) {
    unsafe {
        if !buf.cast::<u32>().is_aligned() {
            todo!()
        }

        let end = buf.add(len);

        let ret = x.serialize_unchecked(buf);

        if ret != end {
            invalid_ret(std::any::type_name::<X>(), buf, len, end, ret)
        }
    }
}

fn buf_too_small(required: usize, available: usize) -> ! {
    panic!(
        "buffer too small for serialization: {} bytes required, but only {} available",
        required, available
    );
}

#[inline(always)]
fn check_len<X: SerializeUnchecked + ?Sized>(x: &X, cap: usize) -> usize {
    let len = x.serialized_len();

    if len > cap {
        buf_too_small(len, cap);
    }

    len
}

pub trait SerializeInto {
    fn ser<X: SerializeUnchecked + ?Sized>(&mut self, x: &X);
}

impl SerializeInto for [u8] {
    fn ser<X: SerializeUnchecked + ?Sized>(&mut self, x: &X) {
        let len = check_len(x, self.len());

        let buf = NonNull::new(self.as_mut_ptr()).unwrap();

        check_ret(x, buf, len);
    }
}

impl<const N: usize> SerializeInto for [u8; N] {
    fn ser<X: SerializeUnchecked + ?Sized>(&mut self, x: &X) {
        let len = check_len(x, N);

        let buf = NonNull::new(self.as_mut_ptr()).unwrap();

        check_ret(x, buf, len);
    }
}

macro_rules! impl_heap {
    ( $( $typ:ty ),+ $( , )? ) => { $(
        impl SerializeInto for $typ {
            fn ser<X: SerializeUnchecked + ?Sized>(&mut self, x: &X) {
                let len = x.serialized_len();

                let cap = self.capacity() - self.len();

                if len > cap {
                    self.reserve(len - cap);
                }

                let buf = NonNull::new(self.spare_capacity_mut().as_mut_ptr() as *mut u8).unwrap();

                check_ret(x, buf, len);

                unsafe { self.set_len(self.len() + len) };
            }
        }
    )+ };
}

impl_heap!(Vec<u8>, ::bytes::BytesMut);
