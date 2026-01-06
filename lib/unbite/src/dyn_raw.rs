use std::hint::assert_unchecked;

use crate::Raw;
use crate::inner::Inner;

#[must_use]
pub struct DynRaw {
    pub(crate) inner: Inner,
    pub(crate) capacity: usize,
}

impl DynRaw {
    crate::common_impl!(
        self: self;
        inner: self.inner;
        capacity: self.capacity;
    );

    #[inline]
    pub fn new(capacity: usize) -> Self {
        let inner = Inner::new_embedded(capacity);

        Self { inner, capacity }
    }

    #[inline]
    pub fn split_dyn_off(&mut self, at: usize) -> DynRaw {
        assert!(at <= self.capacity);

        let inner = unsafe { self.inner.split_off_unchecked(at) };

        let capacity = self.capacity - at;
        self.capacity = at;

        DynRaw { inner, capacity }
    }

    #[inline]
    pub fn split_dyn_to(&mut self, at: usize) -> DynRaw {
        assert!(at <= self.capacity);

        let inner = unsafe { self.inner.split_to_unchecked(at) };

        self.capacity -= at;

        DynRaw {
            inner,
            capacity: at,
        }
    }

    #[inline]
    pub fn split_back<const N: usize>(&mut self) -> Raw<N> {
        assert!(N <= self.capacity);

        self.capacity -= N;

        let inner = unsafe { self.inner.split_off_unchecked(self.capacity) };

        Raw { inner }
    }

    #[inline]
    pub fn split_front<const N: usize>(&mut self) -> Raw<N> {
        assert!(N <= self.capacity);

        self.capacity -= N;

        let inner = unsafe { self.inner.split_to_unchecked(N) };

        Raw { inner }
    }

    #[inline]
    pub fn unsplit_dyn_back(&mut self, r: DynRaw) {
        assert!(self.can_unsplit_dyn_raw_back(&r));

        r.inner.drop_non_deallocating();

        unsafe {
            self.capacity = self.capacity.unchecked_add(r.capacity);
        }
    }

    #[inline]
    pub fn unsplit_dyn_front(&mut self, l: DynRaw) {
        assert!(l.can_unsplit_dyn_raw_back(self));

        self.inner.bytes = l.inner.bytes;

        l.inner.drop_non_deallocating();

        unsafe {
            self.capacity = self.capacity.unchecked_add(l.capacity);
        }
    }

    #[inline]
    pub fn unsplit_back<const N: usize>(&mut self, r: Raw<N>) {
        assert!(self.can_unsplit_raw_back(&r));

        r.inner.drop_non_deallocating();

        unsafe {
            self.capacity = self.capacity.unchecked_add(N);
            assert_unchecked(self.capacity >= N);
        }
    }

    #[inline]
    pub fn unsplit_front<const N: usize>(&mut self, l: Raw<N>) {
        assert!(self.can_unsplit_raw_front(&l));

        self.inner.bytes = l.inner.bytes;

        l.inner.drop_non_deallocating();

        unsafe {
            self.capacity = self.capacity.unchecked_add(N);
            assert_unchecked(self.capacity >= N);
        }
    }
}
