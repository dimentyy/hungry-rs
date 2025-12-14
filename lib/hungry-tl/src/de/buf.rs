use std::marker::PhantomData;
use std::slice;

use crate::de::{Deserialize, Error};

#[derive(Clone)]
pub struct Buf<'a> {
    pub(crate) ptr: *const u8,
    pub(crate) len: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Buf<'a> {
    #[inline(always)]
    pub fn new(slice: &'a [u8]) -> Self {
        Self {
            ptr: slice.as_ptr(),
            len: slice.len(),
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn truncate(&mut self, len: usize) {
        if self.len > len {
            self.len = len;
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }

    #[inline(always)]
    pub unsafe fn advance_unchecked(&mut self, n: usize) -> *const u8 {
        let ptr = self.ptr;

        unsafe {
            self.ptr = self.ptr.add(n);
            self.len = self.len.unchecked_sub(n);
        }

        ptr
    }

    #[inline(always)]
    pub fn check_len(&mut self, n: usize) -> Result<(), Error> {
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

    pub fn de<X: Deserialize>(&mut self) -> Result<X, Error> {
        let len = self.len;

        let x = X::deserialize(self)?;

        // TODO: proper checks
        assert_eq!(x.serialized_len(), len - self.len);

        Ok(x)
    }
}
