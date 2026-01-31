//! THIS IS WORK IN PROGRESS!!!
#![allow(warnings, clippy::all, clippy::pedantic, clippy::nursery)]

use std::collections::VecDeque;
use std::fmt;
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;

use crate::sender::{Messages, Sender, SenderError};
use crate::transport::Transport;
use crate::{mtproto, tl};

use tl::mtproto::enums;

#[derive(Debug)]
pub enum HandleError {
    Sender(SenderError),

    Todo(&'static str),
}

impl fmt::Display for HandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use HandleError::*;

        f.write_str("handle error: ")?;

        match self {
            Sender(err) => err.fmt(f),
            Todo(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for HandleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use HandleError::*;

        match self {
            Sender(err) => Some(err),
            _ => None,
        }
    }
}

struct Request {
    msg: mtproto::Msg,

    tx: oneshot::Sender<tl::Object>,
}

pub struct Handle<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    sender: Sender<T, R, W>,

    requests: VecDeque<Request>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Handle<T, R, W> {
    pub fn new(sender: Sender<T, R, W>) -> Self {
        Self {
            sender,

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
        self.send_rpc_result(res.req_msg_id, res.res_object);
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), HandleError>> {
        use HandleError::*;

        let output = ready!(self.sender.poll(cx)).map_err(Sender)?;

        match output {
            Messages::Msg(buf_msg) => match mtproto::Message::deserialize(buf_msg).unwrap() {
                mtproto::Message::Object { msg, obj } => self.handle_object(msg, obj),
                mtproto::Message::RpcResult { msg, res } => self.handle_result(msg, res),
            },
            Messages::MsgContainer(_msg, container) => {
                let mut res = Vec::with_capacity(container.len());

                for buf_msg in container {
                    res.push(mtproto::Message::deserialize(buf_msg).unwrap());
                }

                for res in res {
                    match res {
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
