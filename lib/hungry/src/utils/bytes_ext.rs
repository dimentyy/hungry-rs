use std::mem;

use bytes::BytesMut;

macro_rules! unsplit_checked {
    ($buffer:expr, $other:expr, $( $arg:tt )+ $(,)?) => {{
        assert!(
            $buffer.can_unsplit(&$other) && (!$buffer.has_spare_capacity() || $other.is_empty()),
            $( $arg )+
        );

        $buffer.unsplit($other)
    }};

    (reverse: $buffer:expr, $other:expr, $( $arg:tt )+ $(,)?) => {
        unsplit_checked!(mem::replace($buffer, $other), $other, $( $arg )+)
    }
}

pub(crate) use unsplit_checked;

pub(crate) trait BytesMutExt {
    fn end_ptr(&self) -> *const u8;

    fn set_zero_len(&mut self);

    unsafe fn set_full_len(&mut self);

    fn unsplit_reverse(&mut self, other: BytesMut);

    fn can_unsplit(&self, other: &BytesMut) -> bool;

    // fn split_right(&mut self, index: usize) -> BytesMut;
    //
    // fn split_left(&mut self, index: usize) -> BytesMut;

    fn spare_capacity_len(&self) -> usize;

    fn has_spare_capacity(&self) -> bool;
}

impl BytesMutExt for BytesMut {
    #[inline]
    fn end_ptr(&self) -> *const u8 {
        self.as_ptr().wrapping_add(self.capacity())
    }

    #[inline]
    fn set_zero_len(&mut self) {
        unsafe { self.set_len(0) }
    }

    #[inline]
    unsafe fn set_full_len(&mut self) {
        unsafe { self.set_len(self.capacity()) }
    }

    #[inline]
    fn unsplit_reverse(&mut self, mut other: BytesMut) {
        mem::swap(self, &mut other);
        self.unsplit(other)
    }

    #[inline]
    fn can_unsplit(&self, other: &BytesMut) -> bool {
        self.end_ptr() == other.as_ptr()
    }

    // fn split_right(&mut self, index: usize) -> BytesMut {
    //     unsafe {
    //         let len = self.len();
    //         self.set_len(self.capacity());
    //
    //         let mut other = self.split_off(index);
    //
    //         self.set_len(len.min(self.capacity()));
    //         other.set_len(len.saturating_sub(index));
    //
    //         other
    //     }
    // }
    //
    // fn split_left(&mut self, index: usize) -> BytesMut {
    //     unsafe {
    //         let len = self.len();
    //         self.set_len(self.capacity());
    //
    //         let mut other = self.split_to(index);
    //
    //         other.set_len(len.min(other.capacity()));
    //         self.set_len(len.saturating_sub(index));
    //
    //         other
    //     }
    // }

    #[inline]
    fn spare_capacity_len(&self) -> usize {
        self.capacity() - self.len()
    }

    #[inline]
    fn has_spare_capacity(&self) -> bool {
        self.spare_capacity_len() > 0
    }
}
