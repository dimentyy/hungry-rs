use std::ptr::NonNull;

use crate::Raw;

pub struct Buf<const N: usize> {
    pub(crate) raw: Raw<N>,
    pub(crate) len: usize,
}

impl<const N: usize> From<Raw<N>> for Buf<N> {
    #[inline]
    fn from(value: Raw<N>) -> Self {
        value.into_buf()
    }
}

impl<const N: usize> Buf<N> {
    crate::common_impl!(
        self: self;
        inner: self.raw.inner;
        len: self.len;
        const_capacity: N;
    );

    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            raw: Raw::new(),
            len: 0,
        }
    }

    #[inline]
    pub fn from_raw(raw: Raw<N>) -> Self {
        Self { raw, len: 0 }
    }

    #[inline]
    pub fn split_raw<const L: usize, const R: usize>(self) -> (Raw<L>, Raw<R>) {
        self.raw.split()
    }

    #[inline]
    pub fn split<const L: usize, const R: usize>(self) -> (Buf<L>, Buf<R>) {
        let (l, r) = self.raw.split();

        (
            Buf {
                raw: l,
                len: self.len.min(L),
            },
            Buf {
                raw: r,
                len: self.len.saturating_sub(L),
            },
        )
    }

    #[inline]
    pub fn unsplit_raw<const L: usize, const R: usize>(l: Raw<L>, r: Raw<R>) -> Self {
        Self {
            raw: Raw::unsplit(l, r),
            len: 0,
        }
    }
}
