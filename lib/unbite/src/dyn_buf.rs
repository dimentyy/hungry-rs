use std::hint::assert_unchecked;
use std::ptr::NonNull;

use crate::{Buf, DynRaw, Raw};

#[must_use]
pub struct DynBuf {
    pub(crate) raw: DynRaw,
    pub(crate) len: usize,
}

impl DynBuf {
    crate::common_impl!(
        self: self;
        inner: self.raw.inner;
        len: self.len;
        capacity: self.raw.capacity;
    );

    #[inline]
    pub fn new(capacity: usize) -> Self {
        let raw = DynRaw::new(capacity);

        Self { raw, len: 0 }
    }

    #[inline]
    pub fn split_raw_back<const N: usize>(&mut self) -> Raw<N> {
        self.len = self.len.min(N);

        self.raw.split_back()
    }

    #[inline]
    pub fn split_raw_front<const N: usize>(&mut self) -> Raw<N> {
        self.len = self.len.saturating_sub(N);

        self.raw.split_front()
    }

    #[inline]
    pub fn unsplit_raw_back<const N: usize>(&mut self, r: Raw<N>) {
        self.raw.unsplit_back(r);

        unsafe {
            assert_unchecked(self.spare_capacity_len() >= N);
        }
    }

    #[inline]
    pub fn unsplit_raw_front<const N: usize>(&mut self, l: Raw<N>) {
        self.raw.unsplit_front(l);

        if N > 0 {
            self.len = 0;
        }
    }

    #[inline]
    pub fn unsplit_buf_back<const N: usize>(&mut self, r: Buf<N>) {
        self.raw.unsplit_back(r.raw);

        if self.len == self.raw.capacity {
            self.len += r.len
        }
    }

    #[inline]
    pub fn unsplit_buf_front<const N: usize>(&mut self, l: Buf<N>) {
        self.raw.unsplit_front(l.raw);

        if l.len == N {
            unsafe {
                self.len = self.len.unchecked_add(l.len);
                assert_unchecked(self.len >= N);
            }
        } else {
            self.len = l.len;
        }
    }

    #[inline]
    pub fn unsplit_back(&mut self, r: DynBuf) {
        self.raw.unsplit_dyn_back(r.raw);

        if self.len == self.raw.capacity {
            self.len += r.len
        }
    }

    #[inline]
    pub fn unsplit_front(&mut self, l: DynBuf) {
        if l.len == l.raw.capacity {
            self.len += l.len;
        } else {
            self.len = l.len;
        }

        self.raw.unsplit_dyn_front(l.raw);
    }

    #[inline]
    pub fn split(&mut self) -> DynRaw {
        self.raw.split_dyn_off(self.len)
    }

    #[inline]
    pub fn split_off(&mut self, at: usize) -> DynBuf {
        let raw = self.raw.split_dyn_off(at);

        let len = self.len().saturating_sub(at);
        self.len = self.len.min(at);

        DynBuf { raw, len }
    }

    #[inline]
    pub fn split_to(&mut self, at: usize) -> DynBuf {
        let raw = self.raw.split_dyn_to(at);

        let len = self.len().min(at);
        self.len = self.len.saturating_sub(at);

        DynBuf { raw, len }
    }
}
