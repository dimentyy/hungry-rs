use std::{fmt, ops, ptr};

use crate::hex;

/// # bytes
///
/// Basic bare type. It is an alias of the string type,
/// with the difference that the value may contain arbitrary
/// byte sequences, including invalid UTF-8 sequences.
///
/// When computing crc32 for a constructor or method it is
/// necessary to replace all byte types with string types.
///
/// ---
///
/// https://core.telegram.org/type/bytes
#[must_use]
#[repr(transparent)]
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    #[inline]
    pub const unsafe fn from_ref(r: &Vec<u8>) -> &Self {
        // SAFETY: `Bytes` is `#[repr(transparent)]` over `Vec<u8>`.
        unsafe { &*ptr::from_ref(r).cast() }
    }

    #[inline]
    pub const unsafe fn from_mut(r: &mut Vec<u8>) -> &mut Self {
        // SAFETY: `Bytes` is `#[repr(transparent)]` over `Vec<u8>`.
        unsafe { &mut *ptr::from_mut(r).cast() }
    }
}

impl AsRef<[u8]> for Bytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsMut<[u8]> for Bytes {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl fmt::Debug for Bytes {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hex::bytes_fmt(self.0.as_ref(), f)
    }
}

impl ops::Deref for Bytes {
    type Target = Vec<u8>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Bytes {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for Bytes {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl PartialEq<Vec<u8>> for Bytes {
    #[inline]
    fn eq(&self, other: &Vec<u8>) -> bool {
        &self.0 == other
    }
}

impl PartialEq<Bytes> for Vec<u8> {
    #[inline]
    fn eq(&self, other: &Bytes) -> bool {
        self == &other.0
    }
}

impl PartialEq<[u8]> for Bytes {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.0.as_slice() == other
    }
}

impl PartialEq<Bytes> for [u8] {
    #[inline]
    fn eq(&self, other: &Bytes) -> bool {
        self == other.0.as_slice()
    }
}
