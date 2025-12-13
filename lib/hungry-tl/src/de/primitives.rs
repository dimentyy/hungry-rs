use crate::de::{DeserializeInfallible, DeserializeUnchecked, Error};
use crate::{FALSE, TRUE};

impl DeserializeInfallible for u32 {
    #[inline]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        Self::from_le(unsafe { *(buf as *const Self) })
    }
}

impl DeserializeInfallible for i32 {
    #[inline]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        Self::from_le(unsafe { *(buf as *const Self) })
    }
}

impl DeserializeInfallible for i64 {
    #[inline]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        Self::from_le(unsafe { (buf as *const Self).read_unaligned() })
    }
}

impl DeserializeInfallible for f64 {
    #[inline]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        Self::from_le_bytes(unsafe { (buf as *const [u8; 8]).read_unaligned() })
    }
}

impl DeserializeUnchecked for bool {
    #[inline]
    unsafe fn deserialize_unchecked(buf: *const u8) -> Result<Self, Error> {
        match unsafe { u32::deserialize_infallible(buf) } {
            TRUE => Ok(true),
            FALSE => Ok(false),
            id => Err(Error::UnexpectedConstructor { id }),
        }
    }
}
