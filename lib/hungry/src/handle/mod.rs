//! THIS IS WORK IN PROGRESS!!!
#![allow(warnings, clippy::all, clippy::pedantic, clippy::nursery)]

use std::collections::VecDeque;
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;

use crate::sender::{Sender, SenderError, SenderOutput};
use crate::transport::Transport;
use crate::{mtproto, tl};

use tl::mtproto::enums;

#[derive(Debug)]
pub enum HandleError {
    Sender(SenderError),

    Todo(&'static str),
}

struct Request {
    msg: mtproto::Msg,

    tx: oneshot::Sender<tl::Object>,
}

pub struct Handle<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    sender: Sender<T, R, W>,

    seq_nos: mtproto::SeqNos,
    msg_ids: mtproto::ServerMsgIds,

    requests: VecDeque<Request>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Handle<T, R, W> {
    pub fn new(sender: Sender<T, R, W>) -> Self {
        Self {
            sender,

            seq_nos: mtproto::SeqNos::new(),
            msg_ids: mtproto::ServerMsgIds::new(1024),

            requests: VecDeque::new(),
        }
    }

    pub fn invoke<F: FnOnce(&mut tl::ser::Buf)>(
        &mut self,
        len: usize,
        f: F,
    ) -> oneshot::Receiver<tl::Object> {
        let msg = self.sender.invoke(len, f).msg;

        let (tx, rx) = oneshot::channel();

        let request = Request { msg, tx };

        self.requests.push_back(request);

        rx
    }

    #[inline]
    fn validate_msg(
        &mut self,
        msg: mtproto::Msg,
        content_related: bool,
        is_response: bool,
    ) -> Result<(), mtproto::MsgError> {
        msg.validate(
            &mut self.msg_ids,
            &mut self.seq_nos,
            std::time::SystemTime::now(),
            content_related,
            is_response,
        )
    }

    fn send_rpc_result(&mut self, req_msg_id: i64, res: tl::Object) {
        let pos = self
            .requests
            .iter()
            .position(|x| x.msg.msg_id == req_msg_id);

        let request = self.requests.remove(pos.unwrap()).unwrap();

        request.tx.send(res).unwrap();
    }

    fn handle_object(&mut self, msg: mtproto::Msg, object: tl::Object) {
        use tl::Object::*;

        match object {
            mtproto_Pong(enums::Pong::Pong(ref pong)) => {
                self.send_rpc_result(pong.msg_id, object);
            }
            mtproto_FutureSalts(enums::FutureSalts::FutureSalts(ref future_salts)) => {
                self.send_rpc_result(future_salts.req_msg_id, object);
            }
            object => {
                // Should be an update.
                dbg!(object);
            }
        }
    }

    fn handle_result(&mut self, msg: mtproto::Msg, res: mtproto::RpcResult) {
        self.send_rpc_result(res.req_msg_id, res.object);
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), HandleError>> {
        use HandleError::*;

        let output = ready!(self.sender.poll(cx)).map_err(Sender)?;

        match output {
            SenderOutput::Message(message) => match message {
                mtproto::Message::Object { msg, obj } => self.handle_object(msg, obj),
                mtproto::Message::RpcResult { msg, res } => self.handle_result(msg, res),
            },
            SenderOutput::MsgContainer(_msg, messages) => {
                for message in messages {
                    match message {
                        mtproto::Message::Object { msg, obj } => self.handle_object(msg, obj),
                        mtproto::Message::RpcResult { msg, res } => self.handle_result(msg, res),
                    }
                }
            }
        }

        cx.waker().wake_by_ref();

        Poll::Pending
    }
}
