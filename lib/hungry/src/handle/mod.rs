#![allow(warnings, clippy::all, clippy::pedantic, clippy::nursery)]

use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::sender::{Sender, SenderError};
use crate::transport::Transport;
use crate::{mtproto, tl, unpack};

#[derive(Debug, Eq, PartialEq)]
pub enum MsgValidationError {
    SeqNo,
    MsgId(mtproto::MsgIdError),
}

#[derive(Debug)]
pub enum HandleError {
    Sender(SenderError),

    Todo(&'static str),
}

pub struct Handle<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    sender: Sender<T, R, W>,

    seq_nos: mtproto::SeqNos,
    msg_ids: mtproto::ServerMsgIds,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Handle<T, R, W> {
    pub fn new(sender: Sender<T, R, W>) -> Self {
        Self {
            sender,

            seq_nos: mtproto::SeqNos::new(),
            msg_ids: mtproto::ServerMsgIds::new(1024),
        }
    }

    #[inline]
    fn validate_msg(
        &mut self,
        msg: mtproto::Msg,
        content_related: bool,
    ) -> Result<mtproto::MsgIdModulus, mtproto::MsgError> {
        msg.validate(
            &mut self.msg_ids,
            &mut self.seq_nos,
            std::time::SystemTime::now(),
            content_related,
        )
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), HandleError>> {
        use HandleError::*;

        let mut buf = ready!(self.sender.poll(cx)).map_err(Sender)?;

        let id = u32::from_le_bytes(*buf.take_exactly().map_err(|_| Todo("too small buffer"))?);

        if id == tl::MSG_CONTAINER {
            let mut msg_container = unpack::MsgContainer::new(buf).unwrap();

            for msg in msg_container {
                let msg = msg.unwrap();
            }
        }

        Poll::Pending
    }
}
