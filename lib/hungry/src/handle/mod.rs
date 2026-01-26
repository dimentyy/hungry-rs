use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::sender::{Sender, SenderError};
use crate::transport::Transport;
use crate::{mtproto, tl, unpack};

#[derive(Debug)]
pub enum HandleError {
    Sender(SenderError),

    Todo(&'static str),
}

pub struct Handle<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    sender: Sender<T, R, W>,

    seq_nos: mtproto::SeqNos,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Handle<T, R, W> {
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), HandleError>> {
        use HandleError::*;

        let mut buf = ready!(self.sender.poll(cx)).map_err(Sender)?;

        let id = u32::from_le_bytes(*buf.take_exactly().map_err(|_| Todo("too small buffer"))?);

        if id == tl::MSG_CONTAINER {
            let mut msg_container = unpack::MsgContainer::new(buf);
        }

        Poll::Pending
    }
}
