use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::slice;

use crate::ser::SerializeUnchecked;

#[must_use]
pub struct Buf<'a> {
    ptr: NonNull<u8>,
    len: usize,
    cap: usize,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Buf<'a> {
    #[inline]
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self {
            ptr: NonNull::from_ref(slice).cast(),
            len: 0,
            cap: slice.len(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn uninit(slice: &'a mut [MaybeUninit<u8>]) -> Self {
        Self {
            ptr: NonNull::from_ref(slice).cast(),
            len: 0,
            cap: slice.len(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    #[must_use]
    pub fn as_non_null(&self) -> NonNull<u8> {
        self.ptr
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    #[inline]
    #[must_use]
    pub fn spare_capacity_len(&self) -> usize {
        self.cap - self.len
    }

    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr().cast_const(), self.len) }
    }

    #[inline]
    #[must_use]
    pub fn as_mut_slice(&self) -> &'a mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    pub fn ser<X: SerializeUnchecked + ?Sized>(&mut self, x: &X) {
        let len = x.serialized_len();

        if len > self.spare_capacity_len() {
            buf_too_small(len, self.len);
        }

        unsafe {
            let ptr = self.ptr.add(self.len);

            let end = ptr.add(len);

            let ret = x.serialize_unchecked(ptr);

            if ret != end {
                invalid_ret(std::any::type_name_of_val(x), ptr, len, end, ret)
            }

            self.len = self.len.unchecked_add(len);
        }
    }
}

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
