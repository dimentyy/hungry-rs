use crate::de::{Buf, DeserializeHybrid, DeserializeInfallible, Error};

impl DeserializeHybrid for crate::Bytes {
    const HYBRID_DESERIALIZATION_UNCHECKED_UNTIL: usize = 4;

    unsafe fn deserialize_hybrid(buf: &mut Buf) -> Result<Self, Error> {
        unsafe {
            let len = *buf.ptr;

            let (src, len) = if len <= 253 {
                let len = len as usize;

                (buf.advance((len + 4) & !3)?.add(1), len)
            } else {
                let len = (u32::deserialize_infallible(buf.ptr) >> 8) as usize;

                (buf.advance((len + 7) & !3)?.add(4), len)
            };

            let mut vec = Vec::with_capacity(len);
            std::ptr::copy_nonoverlapping(src, vec.as_mut_ptr(), len);
            vec.set_len(len);
            Ok(vec)
        }
    }
}

impl DeserializeHybrid for String {
    const HYBRID_DESERIALIZATION_UNCHECKED_UNTIL: usize = 4;

    unsafe fn deserialize_hybrid(buf: &mut Buf) -> Result<Self, Error> {
        match String::from_utf8(unsafe { crate::Bytes::deserialize_hybrid(buf)? }) {
            Ok(s) => Ok(s),
            Err(_) => Err(Error::InvalidUtf8String),
        }
    }
}
