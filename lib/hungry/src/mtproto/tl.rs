use crate::tl;
use hungry_tl::SerializedLen;

use tl::de::DeserializeInfallible;

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

pub struct MsgContainer {
    pub messages: Vec<Message>,
}
