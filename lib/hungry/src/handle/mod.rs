#![allow(warnings, clippy::all)]

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

    fn validate_msg(
        &mut self,
        seq_no: mtproto::SeqNo,
        msg_id: mtproto::MsgId,
        content_related: bool,
    ) -> Result<mtproto::MsgIdModulus, MsgValidationError> {
        if content_related {
            if seq_no & 1 == 0 {
                todo!()
            }

            if seq_no != self.seq_nos.get_content_related() {
                todo!()
            }
        } else {
            if seq_no & 1 == 1 {
                todo!()
            }

            if seq_no != self.seq_nos.non_content_related() {
                todo!()
            }
        };

        let modulus = self
            .msg_ids
            .validate(msg_id, std::time::SystemTime::now())
            .map_err(MsgValidationError::MsgId)?;

        Ok(modulus)
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
