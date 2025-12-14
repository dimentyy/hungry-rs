macro_rules! big_int {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl crate::ser::SerializeUnchecked for $typ {
            #[inline(always)]
            unsafe fn serialize_unchecked(&self, buf: *mut u8) -> *mut u8 {
                unsafe {
                    std::ptr::copy_nonoverlapping(self.as_ptr(), buf, $len);

                    buf.add($len)
                }
            }
        }
    )+ };
}

big_int!(
    crate::Int128 => 16,
    crate::Int256 => 32,
);
