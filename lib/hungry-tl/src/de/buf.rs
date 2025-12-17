use std::marker::PhantomData;
use std::ptr::NonNull;
use std::{hint, slice};

use crate::de::{Deserialize, EndOfBufferError, Error};

#[derive(Clone)]
pub struct Buf<'a> {
    pub(crate) ptr: NonNull<u8>,
    pub(crate) len: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Buf<'a> {
    #[inline(always)]
    pub fn new(slice: &'a [u8]) -> Self {
        let ptr = unsafe { NonNull::new_unchecked(slice.as_ptr().cast_mut()) };
        
        if !ptr.cast::<u32>().is_aligned() {
            todo!()
        }

        Self {
            ptr,
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
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    #[inline(always)]
    pub fn check_len(&mut self, n: usize) -> Result<(), EndOfBufferError> {
        if self.len < n {
            return Err(EndOfBufferError {});
        }

        Ok(())
    }

    #[inline]
    pub fn advance(&mut self, n: usize) -> Result<NonNull<u8>, EndOfBufferError> {
        self.check_len(n)?;

        let ptr = self.ptr;

        unsafe { self.advance_unchecked(n) };

        Ok(ptr)
    }

    #[inline(always)]
    pub unsafe fn advance_unchecked(&mut self, n: usize) -> NonNull<u8> {
        unsafe {
            hint::assert_unchecked(self.len >= n);

            let ptr = self.ptr;

            self.len = self.len.unchecked_sub(n);
            self.ptr = self.ptr.add(n);

            ptr
        }
    }

    pub fn de<X: Deserialize>(&mut self) -> Result<X, Error> {
        let len = self.len;

        let x = X::deserialize(self)?;

        // TODO: proper checks
        assert_eq!(x.serialized_len(), len - self.len);

        Ok(x)
    }
}
