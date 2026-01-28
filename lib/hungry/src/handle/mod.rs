//! THIS IS WORK IN PROGRESS!!!
#![allow(warnings, clippy::all, clippy::pedantic, clippy::nursery)]

use std::collections::VecDeque;
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;

use crate::sender::{Sender, SenderError};
use crate::transport::Transport;
use crate::{mtproto, tl, unpack};

#[derive(Debug)]
pub enum HandleError {
    Sender(SenderError),

    Todo(&'static str),
}

struct Request {
    msg: mtproto::Msg,

    tx: oneshot::Sender<Vec<u8>>,
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
    ) -> oneshot::Receiver<Vec<u8>> {
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

    fn send_rpc_result(&mut self, req_msg_id: i64, bytes: Vec<u8>) {
        let pos = self
            .requests
            .iter()
            .position(|x| x.msg.msg_id == req_msg_id);

        let request = self.requests.remove(pos.unwrap()).unwrap();

        request.tx.send(bytes).unwrap();
    }

    // TODO: do NOT allocate.
    fn process_msg(&mut self, msg: mtproto::Msg, mut buf: tl::de::Buf) {
        let id: u32 = buf.de().unwrap();

        eprintln!("{id:#010x}");

        match id {
            tl::RPC_RESULT => {
                let req_msg_id: i64 = buf.de().unwrap();

                self.send_rpc_result(req_msg_id, buf.as_slice().to_vec());
            }
            _ => {

            }
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), HandleError>> {
        use HandleError::*;

        let res = ready!(self.sender.poll(cx)).map_err(Sender)?;

        cx.waker().wake_by_ref();

        let bytes = res.as_slice().to_vec();
        let mut res = tl::de::Buf::new(&bytes);

        let msg: mtproto::BytesMsg = res.de().map_err(|_| Todo("top msg unpack err"))?;

        let mut buf = res.clone();

        let id = u32::from_le_bytes(*buf.take_exactly().map_err(|_| Todo("too small buffer"))?);

        if id == tl::MSG_CONTAINER {
            let mut msg_container =
                unpack::MsgContainer::new(buf).map_err(|_| Todo("msg container unpack failed"))?;

            for msg in msg_container {
                let mtproto::BufMsg { msg, buf } = msg.map_err(|_| Todo("msg unpack failed"))?;

                self.process_msg(msg, buf);
            }

            return Poll::Pending
        }

        self.process_msg(msg.msg, res);

        Poll::Pending
    }
}
