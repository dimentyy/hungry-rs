use std::task::{Context, Poll, ready};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::sender::{Sender, SenderError};
use crate::transport::Transport;

#[derive(Debug)]
pub enum ClientError {
    Sender(SenderError),

    Todo(&'static str),
}

pub struct Client<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    sender: Sender<T, R, W>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Client<T, R, W> {
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ClientError>> {
        use ClientError::*;

        let mut buf = ready!(self.sender.poll(cx)).map_err(Sender)?;

        Poll::Pending
    }
}
