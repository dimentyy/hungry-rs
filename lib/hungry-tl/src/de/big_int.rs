use crate::{Int128, Int256, de::DeserializeInfallible};

macro_rules! big_int {
    ( $( $typ:ty => $len:expr ),+ $(,)? ) => { $(
        impl DeserializeInfallible for $typ {
            #[inline]
            unsafe fn deserialize_infallible(buf: *const u8) -> Self {
                unsafe { (buf as *const [u8; $len]).read_unaligned() }
            }
        }
    )+ };
}

big_int!(
    Int128 => 16,
    Int256 => 32,
);
