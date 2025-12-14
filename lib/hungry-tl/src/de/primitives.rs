use crate::de::{DeserializeInfallible, DeserializeUnchecked, Error};
use crate::{FALSE, TRUE};

macro_rules! impls {
    ( $buf:ident; $( $typ:ty: $val:expr ),+ $( , )? ) => { $(
        impl DeserializeInfallible for $typ {
            #[inline(always)]
            unsafe fn deserialize_infallible($buf: *const u8) -> Self {
                $val
            }
        }
    )+ };
}

impls!(buf;
    u32: Self::from_le(unsafe { *(buf as *const Self) }),
    i32: Self::from_le(unsafe { *(buf as *const Self) }),
    i64: Self::from_le(unsafe { (buf as *const Self).read_unaligned() }),
    f64: Self::from_bits(unsafe { i64::deserialize_infallible(buf) } as u64)
);

impl DeserializeUnchecked for bool {
    #[inline(always)]
    unsafe fn deserialize_unchecked(buf: *const u8) -> Result<Self, Error> {
        match unsafe { u32::deserialize_infallible(buf) } {
            TRUE => Ok(true),
            FALSE => Ok(false),
            _ => Err(Error::UnexpectedConstructor),
        }
    }
}
