// use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::sender::Sender;
use crate::transport::Transport;

pub struct Client<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    sender: Sender<T, R, W>
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Client<T, R, W> {
    // pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<>> {
    // 
    // }
}
