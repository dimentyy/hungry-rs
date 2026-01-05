use std::hint::assert_unchecked;
use std::ptr::NonNull;

use crate::{Buf, DynRaw, Raw};

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
}
