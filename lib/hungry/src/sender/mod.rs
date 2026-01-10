mod container;

use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::mtproto;
use crate::reader::{Reader, ReaderError};
use crate::transport::Transport;
use crate::writer::{QueuedWriter, WriterError};

pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),
}

impl From<ReaderError> for SenderError {
    #[inline]
    fn from(value: ReaderError) -> Self {
        Self::Reader(value)
    }
}

impl From<WriterError> for SenderError {
    #[inline]
    fn from(value: WriterError) -> Self {
        Self::Writer(value)
    }
}

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: mtproto::AuthKey,
    session: mtproto::Session,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Sender<T, R, W> {
    pub fn new(
        reader: Reader<R, T>,
        writer: QueuedWriter<W, T>,

        auth_key: mtproto::AuthKey,
        session: mtproto::Session,
    ) -> Self {
        Self {
            reader,
            writer,

            auth_key,
            session,
        }
    }

    fn push_completed_writer_buffer(&mut self, buffer: unbite::DynBuf) {}

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SenderError>> {
        if !self.writer.is_empty() {
            loop {
                let Poll::Ready(buffer) = self.writer.poll(cx)? else {
                    break;
                };

                self.push_completed_writer_buffer(buffer);
            }
        }

        todo!()
    }
}
