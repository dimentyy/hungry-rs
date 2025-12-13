use crate::tl;

use tl::de::{Buf, Deserialize, DeserializeInfallible, Error};
use tl::ser::Serialize;
use tl::ConstSerializedLen;

#[derive(Debug)]
pub struct Msg {
    pub msg_id: i64,
    pub seq_no: i32,
}

impl ConstSerializedLen for Msg {
    const SERIALIZED_LEN: usize = i64::SERIALIZED_LEN + i32::SERIALIZED_LEN;
}

impl Serialize for Msg {
    unsafe fn serialize_unchecked(&self, mut buf: *mut u8) -> *mut u8 {
        unsafe {
            buf = self.msg_id.serialize_unchecked(buf);
            buf = self.seq_no.serialize_unchecked(buf);
            buf
        }
    }
}

impl DeserializeInfallible for Msg {
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        unsafe {
            Self {
                msg_id: i64::deserialize_infallible(buf),
                seq_no: i32::deserialize_infallible(buf.add(8)),
            }
        }
    }
}

/// Containers are messages containing several other messages.
/// Used for the ability to transmit several RPC queries and/or service
/// messages at the same time, using HTTP or even TCP or UDP protocol.
/// A container may only be accepted or rejected by the other party as a whole.
///
/// ---
/// https://core.telegram.org/mtproto/service_messages#containers
pub struct MsgContainer<'a> {
    buf: &'a mut Buf<'a>,
    len: usize,
}

impl<'a> MsgContainer<'a> {
    pub fn new(buf: &'a mut Buf<'a>) -> Result<Self, Error> {
        let len = u32::deserialize_checked(buf)? as usize;

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
