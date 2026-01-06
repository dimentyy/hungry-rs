mod big_int;
mod buf;
mod primitives;
mod string;
mod vec;

use std::ptr::NonNull;

use crate::SerializedLen;

pub use string::string_len;

pub trait SerializeUnchecked: SerializedLen {
    /// Serializes the instance into `buf` without checking its capacity.
    ///
    /// # Safety
    ///
    /// * `buf` must have at least [`serialized_len`] bytes of capacity.
    ///
    /// [`serialized_len`]: SerializedLen::serialized_len
    unsafe fn serialize_unchecked(&self, buf: NonNull<u8>) -> NonNull<u8>;
}

#[inline]
#[track_caller]
pub fn safe<X: SerializeUnchecked + ?Sized>(x: &X, buf: &mut [u8]) {
    #[cold]
    #[inline(never)]
    fn buf_too_small(required: usize, available: usize) -> ! {
        panic!(
            "buffer too small for serialization: {} bytes required, but only {} available",
            required, available
        );
    }

    #[cold]
    #[inline(never)]
    fn invalid_ret(
        type_name: &str,
        buf: NonNull<u8>,
        len: usize,
        end: NonNull<u8>,
        ret: NonNull<u8>,
    ) -> ! {
        let off = unsafe { ret.offset_from(end) };

        panic!(
            "impl `SerializeUnchecked` for `{type_name}` is invalid: \
            expected `serialize_unchecked` to return {end:?} \
            ({buf:?} + {len:#x}), got {ret:?}, off by {off}",
        );
    }

    let ptr = NonNull::from_ref(buf).cast::<u8>();

    let len = x.serialized_len();

    if len > buf.len() {
        buf_too_small(len, buf.len());
    }

    unsafe {
        let end = ptr.add(len);

        let ret = x.serialize_unchecked(ptr);

        if ret != end {
            invalid_ret(std::any::type_name_of_val(x), ptr, len, end, ret)
        }
    }
}
