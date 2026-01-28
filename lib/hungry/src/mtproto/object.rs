use crate::{mtproto, tl};

#[derive(Debug, PartialEq)]
pub struct RpcResult {
    pub req_msg_id: mtproto::MsgId,
    pub object: Box<Object>
}

#[derive(Debug, PartialEq)]
pub enum Object {
    Object(tl::Object),
    RpcResult(RpcResult)
}
