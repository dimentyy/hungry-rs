use crate::tl;

use tl::de::{Buf, Deserialize, DeserializeInfallible, Error};
use tl::SerializedLen;

#[derive(Debug)]
pub struct Message {
    pub msg_id: i64,
    pub seq_no: i32,
    pub length: i32,
}

impl Message {
    #[inline]
    pub fn length(&self) -> usize {
        self.length as usize
    }
}

impl SerializedLen for Message {
    const SERIALIZED_LEN: usize = 16;
}

impl DeserializeInfallible for Message {
    unsafe fn deserialize_infallible(buf: *const u8) -> Self {
        unsafe {
            Self {
                msg_id: i64::deserialize_infallible(buf),
                seq_no: i32::deserialize_infallible(buf.add(8)),
                length: i32::deserialize_infallible(buf.add(12)),
            }
        }
    }
}

/// Containers are messages containing several other messages.
/// Used for the ability to transmit several RPC queries and/or service
/// messages at the same time, using HTTP or even TCP or UDP protocol.
/// A container may only be accepted or rejected by the other party as a whole.
///
/// https://core.telegram.org/mtproto/service_messages#containers
pub struct MsgContainer {
    pub messages: Vec<Message>,
}

impl Deserialize for MsgContainer {
    const MINIMUM_SERIALIZED_LEN: usize = 4;

    unsafe fn deserialize(buf: &mut Buf) -> Result<Self, Error> {
        let len = unsafe { u32::deserialize_infallible(buf.advance_unchecked(4)) as usize };

        let mut messages = Vec::with_capacity(len);

        for _ in 0..len {
            let message = Message::deserialize_checked(buf)?;
            let _ = buf.advance(message.length())?;
            messages.push(message);
        }

        Ok(Self { messages })
    }
}

impl MsgContainer {}
