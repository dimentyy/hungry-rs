use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::reader::Reader;
use crate::transport::Transport;
use crate::writer::Writer;

pub const MAX_LEN: usize = 1024 * (1024 + 2);

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: Writer<W, T>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Sender<T, R, W> {
    pub fn new(transport: T, reader: R, writer: W) -> Self {
        let reader_buffer = BytesMut::with_capacity(MAX_LEN);

        let (reader, writer) = crate::new(transport, reader, reader_buffer, writer);

        Self { reader, writer }
    }
}
