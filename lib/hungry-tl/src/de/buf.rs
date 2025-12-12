use std::marker::PhantomData;
use std::slice;

use crate::de::{DeserializeInfallible, Error};

#[derive(Clone)]
pub struct Buf<'a> {
    pub(super) ptr: *const u8,
    pub(super) len: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Buf<'a> {
    #[inline]
    pub fn new(slice: &'a [u8]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn as_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    #[inline]
    pub unsafe fn advance_unchecked(&mut self, n: usize) -> *const u8 {
        let ptr = self.ptr;

        unsafe {
            self.ptr = self.ptr.add(n);
            self.len = self.len.unchecked_sub(n);
        }

        ptr
    }

    #[inline]
    pub(crate) fn check_len(&mut self, n: usize) -> Result<(), Error> {
        if self.len < n {
            return Err(Error::UnexpectedEndOfBuffer);
        }

        Ok(())
    }

    #[inline]
    pub fn advance(&mut self, n: usize) -> Result<*const u8, Error> {
        self.check_len(n)?;

        let ptr = self.ptr;

        unsafe { self.advance_unchecked(n) };

        Ok(ptr)
    }
    
    pub fn infallible<T: DeserializeInfallible>(&mut self) -> T {
        let ptr = self
            .advance(T::SERIALIZED_LEN)
            .expect("`Buf` to have required length");

        unsafe { T::deserialize_infallible(ptr) }
    }
}
