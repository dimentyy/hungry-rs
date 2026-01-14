#![forbid(unsafe_code, clippy::todo)]

mod error;
mod owned;
mod queued;

use std::io;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use tokio::io::AsyncWrite;

use crate::mtproto;
use crate::transport::{Transport, TransportWrite};

pub use error::WriterError;
pub use owned::{OwnedWrite, OwnedWriteInner};
pub use queued::QueuedWriter;

pub struct Writer<W: AsyncWrite + Unpin, T: Transport> {
    driver: W,
    transport: T::Write,
}

impl<W: AsyncWrite + Unpin, T: Transport> Writer<W, T> {
    #[inline]
    pub(crate) fn new(driver: W, transport: T::Write) -> Self {
        Self { driver, transport }
    }

    fn poll_checked(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<NonZeroUsize>> {
        let n = ready!(Pin::new(&mut self.driver).poll_write(cx, buf))?;

        assert!(
            n <= buf.len(),
            "`tokio::io::AsyncWrite` contract violation by `{}`: \
            reported number of bytes written ({n}) \
            exceeds the buffer length ({})",
            std::any::type_name::<W>(),
            buf.len(),
        );

        let Some(n) = NonZeroUsize::new(n) else {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "wrote 0 bytes",
            )));
        };

        Poll::Ready(Ok(n))
    }

    pub fn single_plain<'a>(
        &'a mut self,
        envelope: T::Envelope,
        header: mtproto::PlainHeader,
        buffer: &'a mut unbite::DynBuf,
        message_id: i64,
    ) -> Single<'a, W, T> {
        mtproto::pack_plain(header, buffer, message_id);

        self.single_impl(buffer, envelope)
    }

    pub fn single<'a>(
        &'a mut self,
        envelope: T::Envelope,
        header: mtproto::EncryptedHeader,
        buffer: &'a mut unbite::DynBuf,
        padding: mtproto::EncryptedPadding,
        auth_key: &mtproto::AuthKey,
        message: mtproto::InternalHeader,
        msg: mtproto::Msg,
    ) -> Single<'a, W, T> {
        mtproto::pack_encrypted(header, buffer, padding, auth_key, message, msg);

        self.single_impl(buffer, envelope)
    }

    fn single_impl<'a>(
        &'a mut self,
        buffer: &'a mut unbite::DynBuf,
        envelope: T::Envelope,
    ) -> Single<'a, W, T> {
        self.transport.pack(buffer, envelope);

        Single {
            writer: self,
            buffer,
            pos: 0,
        }
    }
}

pub struct Single<'a, W: AsyncWrite + Unpin, T: Transport> {
    writer: &'a mut Writer<W, T>,
    buffer: &'a mut unbite::DynBuf,
    pos: usize,
}

impl<W: AsyncWrite + Unpin, T: Transport> Single<'_, W, T> {
    #[inline]
    #[must_use]
    pub fn pos(self) -> usize {
        self.pos
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), WriterError>> {
        loop {
            let buf = &self.buffer.as_slice()[self.pos..];

            if buf.is_empty() {
                return Poll::Ready(Ok(()));
            }

            let n = ready!(self.writer.poll_checked(cx, buf)).map_err(WriterError::Io)?;

            self.pos += n.get();
        }
    }
}
