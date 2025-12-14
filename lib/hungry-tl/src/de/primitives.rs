use std::mem;

use crate::de::{DeserializeInfallible, DeserializeUnchecked, Error};

impl DeserializeInfallible for u32 {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        Self::from_le(unsafe { *(buf as *const Self) })
    }
}

impl DeserializeInfallible for i32 {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        Self::from_le(unsafe { *(buf as *const Self) })
    }
}

impl DeserializeInfallible for i64 {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        Self::from_le(unsafe { (buf as *const Self).read_unaligned() })
    }
}

impl DeserializeInfallible for f64 {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        unsafe { mem::transmute(i64::deserialize_infallible(buf)) }
    }
}

impl DeserializeUnchecked for bool {
    #[inline]
    unsafe fn deserialize_unchecked(buf: *const u8) -> Result<Self, Error> {
        match unsafe { u32::deserialize_infallible(buf) } {
            crate::TRUE => Ok(true),
            crate::FALSE => Ok(false),
            _ => Err(Error::UnexpectedConstructor),
        }
    }
}
