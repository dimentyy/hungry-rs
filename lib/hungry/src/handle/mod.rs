//! THIS IS WORK IN PROGRESS!!!
#![allow(warnings, clippy::all, clippy::pedantic, clippy::nursery)]

use std::collections::VecDeque;
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;

use crate::sender::{Sender, SenderError};
use crate::transport::Transport;
use crate::{mtproto, tl, unpack};

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

    // TODO: do NOT allocate.
    fn process_msg(
        &mut self,
        msg: mtproto::Msg,
        mut buf: tl::de::Buf,
    ) -> Result<Option<tl::Object>, tl::de::Error> {
        let id = u32::from_le_bytes(*buf.peek_exactly()?);

        if id == tl::RPC_RESULT {
            let Ok(_) = buf.advance(4) else {
                unreachable!()
            };

            let req_msg_id: i64 = buf.de_infallible()?;

            let id = u32::from_le_bytes(*buf.peek_exactly()?);

            if id == tl::GZIP_PACKED {
                todo!()
            }

            let object: tl::Object = buf.de()?;

            self.send_rpc_result(req_msg_id, object);

            return Ok(None);
        }

        let object = buf.de()?;

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
                return Ok(Some(object));
            }
        }

        Ok(None)
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<Vec<tl::Object>, HandleError>> {
        use HandleError::*;

        let res = ready!(self.sender.poll(cx)).map_err(Sender)?;

        let bytes = res.as_slice().to_vec();
        let mut res = tl::de::Buf::new(&bytes);

        let msg: mtproto::BytesMsg = res
            .de_infallible()
            .map_err(|_| Todo("top msg unpack err"))?;

        let mut buf = res.clone();

        let id = u32::from_le_bytes(*buf.take_exactly().map_err(|_| Todo("too small buffer"))?);

        let mut objects = Vec::new();

        if id == tl::MSG_CONTAINER {
            let mut msg_container =
                unpack::MsgContainer::new(buf).map_err(|_| Todo("msg container unpack failed"))?;

            for msg in msg_container {
                let mtproto::BufMsg { msg, buf } = msg.map_err(|_| Todo("msg unpack failed"))?;

                if let Some(object) = self.process_msg(msg, buf).map_err(|_| Todo("de err"))? {
                    objects.push(object)
                }
            }
        } else {
            if let Some(object) = self.process_msg(msg.msg, res).map_err(|_| Todo("de err"))? {
                objects.push(object)
            }
        }

        if objects.is_empty() {
            cx.waker().wake_by_ref();

            return Poll::Pending;
        }

        Poll::Ready(Ok(objects))
    }
}
