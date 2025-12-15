use std::ptr::NonNull;

use crate::Bytes;
use crate::de::{Buf, Deserialize, DeserializeInfallible, Error};

impl Deserialize for Bytes {
    fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        buf.check_len(4)?;

        unsafe {
            let len = buf.ptr.read();

            let (src, len) = if len <= 253 {
                let len = len as usize;

                (buf.advance((len + 4) & !3)?.add(1), len)
            } else {
                let len = (u32::deserialize_infallible(buf.ptr) >> 8) as usize;

                (buf.advance((len + 7) & !3)?.add(4), len)
            };

            let mut vec = Vec::with_capacity(len);

            src.copy_to(NonNull::new_unchecked(vec.as_mut_ptr()), len);

            vec.set_len(len);

            Ok(vec)
        }
    }
}

impl Deserialize for String {
    fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        match String::from_utf8(Bytes::deserialize(buf)?) {
            Ok(s) => Ok(s),
            Err(err) => Err(Error::InvalidUtf8String(err.utf8_error())),
        }
    }
}
