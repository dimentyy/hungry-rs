use std::marker::PhantomData;
use std::ptr::NonNull;
use std::{fmt, hint, ptr, slice};

use crate::de::{Deserialize, DeserializeInfallible, EndOfBufferError, Error};

#[must_use]
#[derive(Clone)]
pub struct Buf<'a> {
    pub(crate) ptr: NonNull<u8>,
    pub(crate) len: usize,
    _marker: PhantomData<&'a ()>,
}

impl fmt::Debug for Buf<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tl::de::Buf {{ len: {}, .. }}", self.len)
    }
}

impl<'a> Buf<'a> {
    #[inline(always)]
    pub fn new(slice: &'a [u8]) -> Self {
        let ptr = NonNull::from_ref(slice).cast();

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
    pub fn check_len(&self, n: usize) -> Result<(), EndOfBufferError> {
        if self.len < n {
            return Err(EndOfBufferError);
        }

        Ok(())
    }

    #[inline(always)]
    pub fn advance(&mut self, n: usize) -> Result<NonNull<u8>, EndOfBufferError> {
        self.check_len(n)?;

        let ptr = self.ptr;

        unsafe { self.advance_unchecked(n) };

        Ok(ptr)
    }

    /// # Safety
    ///
    /// * Provided offset `n` must be less or equal to the [`Buf::len`].
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

    #[inline(always)]
    pub fn take_exactly<const N: usize>(&mut self) -> Result<&'a [u8; N], EndOfBufferError> {
        let ptr = self.advance(N)?.cast();

        Ok(unsafe { ptr.as_ref() })
    }

    #[inline(always)]
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], EndOfBufferError> {
        let ptr = self.advance(n)?;

        Ok(unsafe { &*ptr::slice_from_raw_parts(ptr.as_ptr(), n) })
    }

    #[inline(always)]
    pub fn peek_exactly<const N: usize>(&self) -> Result<&'a [u8; N], EndOfBufferError> {
        self.check_len(N)?;

        Ok(unsafe { self.ptr.cast().as_ref() })
    }

    pub fn de<X: Deserialize>(&mut self) -> Result<X, Error> {
        let len = self.len;

        let x = X::deserialize(self)?;

        assert_eq!(x.serialized_len(), len - self.len);

        Ok(x)
    }

    pub fn de_infallible<X: DeserializeInfallible>(&mut self) -> Result<X, EndOfBufferError> {
        let ptr = self.advance(X::SERIALIZED_LEN)?;

        let x = unsafe { X::deserialize_infallible(ptr) };

        Ok(x)
    }
}
