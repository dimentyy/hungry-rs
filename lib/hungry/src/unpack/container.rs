use crate::mtproto::Msg;
use crate::tl;

use tl::ConstSerializedLen;
use tl::de::{Buf, DeserializeInfallible, Error};

/// Containers are messages containing several other messages.
/// Used for the ability to transmit several RPC queries and/or service
/// messages at the same time, using HTTP or even TCP or UDP protocol.
/// A container may only be accepted or rejected by the other party as a whole.
///
/// ---
/// https://core.telegram.org/mtproto/service_messages#containers
pub struct MsgContainer<'a> {
    buf: Buf<'a>,
    len: usize,
}

impl<'a> MsgContainer<'a> {
    pub fn new(mut buf: Buf<'a>) -> Result<Self, Error> {
        let len = buf.de::<u32>()? as usize;

        Ok(Self { buf, len })
    }

    fn deserialize_next_message(&mut self) -> <Self as Iterator>::Item {
        unsafe {
            let ptr = self
                .buf
                .advance(Msg::SERIALIZED_LEN + i32::SERIALIZED_LEN)?;

            let message = Msg::deserialize_infallible(ptr);

            // FIXME: negative length check.
            let bytes = i32::deserialize_infallible(ptr.add(Msg::SERIALIZED_LEN)) as usize;

            let mut buf = self.buf.clone();

            self.buf.advance(bytes)?;

            buf.truncate(bytes);

            Ok((message, buf))
        }
    }
}

impl<'a> Iterator for MsgContainer<'a> {
    type Item = Result<(Msg, Buf<'a>), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;

        Some(self.deserialize_next_message())
    }
}
