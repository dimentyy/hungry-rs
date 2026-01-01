use std::{fmt, ops, ptr};

/// # vector
///
/// A universal vector constructor.
///
/// ``` tl
/// vector {t:Type} # [ t ] = Vector t;
/// ```
///
/// ---
///
/// https://core.telegram.org/constructor/vector
#[must_use]
#[repr(transparent)]
#[derive(Clone, Default, PartialEq, Eq)]
pub struct BareVec<T>(pub Vec<T>);

impl<T> BareVec<T> {
    #[inline]
    pub const unsafe fn from_ref(r: &Vec<T>) -> &Self {
        // SAFETY: `BareVec<T>` is `#[repr(transparent)]` over `Vec<T>`.
        unsafe { &*ptr::from_ref(r).cast() }
    }

    #[inline]
    pub const unsafe fn from_mut(r: &mut Vec<T>) -> &mut Self {
        // SAFETY: `BareVec<V>` is `#[repr(transparent)]` over `Vec<T>`.
        unsafe { &mut *ptr::from_mut(r).cast() }
    }
}

impl<T> AsRef<[T]> for BareVec<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<T> AsMut<[T]> for BareVec<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }
}

impl<T: fmt::Debug> fmt::Debug for BareVec<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl<T> ops::Deref for BareVec<T> {
    type Target = Vec<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ops::DerefMut for BareVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<Vec<T>> for BareVec<T> {
    #[inline]
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}

impl<T: Eq> PartialEq<Vec<T>> for BareVec<T> {
    #[inline]
    fn eq(&self, other: &Vec<T>) -> bool {
        &self.0 == other
    }
}

impl<T: Eq> PartialEq<BareVec<T>> for Vec<T> {
    #[inline]
    fn eq(&self, other: &BareVec<T>) -> bool {
        self == &other.0
    }
}

impl<T: Eq> PartialEq<[T]> for BareVec<T> {
    #[inline]
    fn eq(&self, other: &[T]) -> bool {
        self.0.as_slice() == other
    }
}

impl<T: Eq> PartialEq<BareVec<T>> for [T] {
    #[inline]
    fn eq(&self, other: &BareVec<T>) -> bool {
        self == other.0.as_slice()
    }
}
