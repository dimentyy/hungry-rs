mod buf;
mod bytes;

use std::fmt;
use std::ptr::NonNull;

use crate::{mtproto, tl};

use tl::ConstSerializedLen;
use tl::de::DeserializeInfallible;
use tl::ser::SerializeUnchecked;

pub use buf::{BufMsg, BufMsgError};
pub use bytes::BytesMsg;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NegativeBytesError(pub i32);

impl fmt::Display for NegativeBytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "negative value in the `bytes` field: {}", self.0)
    }
}

impl std::error::Error for NegativeBytesError {}

#[derive(Clone, Debug, PartialEq)]
pub struct RpcResult {
    pub req_msg_id: mtproto::MsgId,
    pub res_object: tl::Object,
}

#[derive(Debug, PartialEq)]
pub enum Message {
    Object { msg: Msg, obj: tl::Object },
    RpcResult { msg: Msg, res: RpcResult },
}

impl Message {
    pub fn deserialize(mut buf_msg: BufMsg<'_>) -> Result<Self, tl::de::Error> {
        if buf_msg.typ != tl::RPC_RESULT {
            let obj = tl::Object::deserialize(buf_msg.typ, &mut buf_msg.buf)?;

            return Ok(Message::Object {
                msg: buf_msg.msg,
                obj,
            });
        }

        let req_msg_id = buf_msg.buf.de_infallible()?;

        let typ = buf_msg.buf.de_infallible()?;

        let object = tl::Object::deserialize(typ, &mut buf_msg.buf)?;

        let res = RpcResult {
            req_msg_id,
            res_object: object,
        };

        Ok(Message::RpcResult {
            msg: buf_msg.msg,
            res,
        })
    }
}

#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Msg {
    pub msg_id: mtproto::MsgId,
    pub seq_no: mtproto::SeqNo,
}

impl ConstSerializedLen for Msg {
    const SERIALIZED_LEN: usize = mtproto::MsgId::SERIALIZED_LEN + mtproto::SeqNo::SERIALIZED_LEN;
}

impl SerializeUnchecked for Msg {
    #[inline(always)]
    unsafe fn serialize_unchecked(&self, mut buf: NonNull<u8>) -> NonNull<u8> {
        // SAFETY: the `SERIALIZED_LEN` is exactly 12;
        // the caller must uphold the safety contract.
        unsafe {
            buf = self.msg_id.serialize_unchecked(buf);
            buf = self.seq_no.serialize_unchecked(buf);
        }

        buf
    }
}

impl DeserializeInfallible for Msg {
    #[inline(always)]
    unsafe fn deserialize_infallible(buf: NonNull<u8>) -> Self {
        // SAFETY: the `SERIALIZED_LEN` is exactly 12;
        // the caller must uphold the safety contract.
        unsafe {
            Self {
                msg_id: i64::deserialize_infallible(buf),
                seq_no: i32::deserialize_infallible(buf.add(8)),
            }
        }
    }
}
