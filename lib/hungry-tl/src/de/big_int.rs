macro_rules! big_int {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl crate::de::DeserializeInfallible for $typ {
            #[inline(always)]
            unsafe fn deserialize_infallible(buf: *const u8) -> Self {
                unsafe { (buf as *const [u8; $len]).read_unaligned() }
            }
        }
    )+ };
}

big_int!(
    crate::Int128 => 16,
    crate::Int256 => 32,
);
