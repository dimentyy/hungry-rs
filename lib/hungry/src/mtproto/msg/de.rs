use std::fmt;

use crate::{mtproto, tl};

use tl::de::DeserializeInfallible;

#[derive(Debug)]
pub enum MsgDeError {
    NegativeLength,
    UnexpectedEndOfBuffer(tl::de::EndOfBufferError),
}

impl fmt::Display for MsgDeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for MsgDeError {}

pub struct MsgDe<'a> {
    pub msg: mtproto::Msg,
    pub buf: tl::de::Buf<'a>,
}

impl<'a> MsgDe<'a> {
    pub fn deserialize(buf: &'_ mut tl::de::Buf<'a>) -> Result<Self, MsgDeError> {
        let ptr = buf.advance(16).map_err(MsgDeError::UnexpectedEndOfBuffer)?;

        unsafe {
            let msg = mtproto::Msg::deserialize_infallible(ptr);
            let len = i32::deserialize_infallible(ptr.add(12));

            let Ok(len) = usize::try_from(len) else {
                return Err(MsgDeError::NegativeLength);
            };

            let buf = tl::de::Buf::new(buf.take(len).map_err(MsgDeError::UnexpectedEndOfBuffer)?);

            Ok(Self { msg, buf })
        }
    }
}
