mod container;

use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};

use container::Container;
use crate::mtproto;
use crate::reader::{Reader, ReaderError};
use crate::transport::Transport;
use crate::writer::{QueuedWriter, WriterError};

pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),
}

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    container: Container<T>,

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

            // FIXME
            container: Container::new(unbite::DynBuf::new(1_000_000)),

            auth_key,
            session,
        }
    }

    fn push_completed_writer_buffer(&mut self, buffer: unbite::DynBuf) {}

    fn take_container(&mut self) -> Container<T> {
        todo!()
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SenderError>> {
        if !self.writer.is_empty() || !self.container.is_empty() {
            loop {
                let Poll::Ready(buffer) = self.writer.poll(cx).map_err(SenderError::Writer)? else {
                    if self.container.is_empty() {
                        break;
                    }

                    let container = self.take_container();

                    continue;
                };

                self.push_completed_writer_buffer(buffer);
            }
        }

        todo!()
    }
}
