use crate::Buf;
use crate::inner::Inner;

pub struct Raw<const N: usize> {
    pub(crate) inner: Inner,
}

impl<const N: usize> Raw<N> {
    crate::common_impl!(
        self: self;
        inner: self.inner;
        const_capacity: N;
    );

    #[inline]
    pub fn new() -> Self {
        let inner = Inner::new_embedded(N);

        Self { inner }
    }
    
    #[inline]
    pub fn into_buf(self) -> Buf<N> {
        Buf { raw: self, len: 0 }
    }

    #[inline]
    pub fn split<const L: usize, const R: usize>(mut self) -> (Raw<L>, Raw<R>) {
        const { assert!(L + R == N) };

        let r = unsafe { self.inner.split_off_unchecked(L) };

        (Raw { inner: self.inner }, Raw { inner: r })
    }

    #[inline]
    pub fn unsplit<const L: usize, const R: usize>(mut l: Raw<L>, r: Raw<R>) -> Self {
        assert!(l.can_unsplit_raw_back(&r));

        unsafe { Raw::unsplit_unchecked(l, r) }
    }

    #[inline]
    pub unsafe fn unsplit_unchecked<const L: usize, const R: usize>(l: Raw<L>, r: Raw<R>) -> Self {
        const { assert!(L + R == N) };

        r.inner.drop_non_deallocating();

        Raw { inner: l.inner }
    }
}
