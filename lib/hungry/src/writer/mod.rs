mod error;

use std::io;
use std::num::NonZeroUsize;
use std::pin::pin;
use std::task::{Context, Poll, ready};

use tokio::io::AsyncWrite;

use crate::mtproto;
use crate::transport::{Transport, TransportWrite};

pub use error::WriterError;

pub trait WriterDriver: AsyncWrite + Unpin {}
impl<T: AsyncWrite + Unpin> WriterDriver for T {}

pub struct Writer<W: WriterDriver, T: Transport> {
    pub driver: W,
    pub transport: T::Write,
}

impl<W: WriterDriver, T: Transport> Writer<W, T> {
    pub(crate) fn new(driver: W, transport: T::Write) -> Self {
        Self { driver, transport }
    }

    #[inline]
    pub fn driver(&mut self) -> &mut W {
        &mut self.driver
    }

    fn poll_checked(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<NonZeroUsize>> {
        let n = ready!(pin!(&mut self.driver).poll_write(cx, buf))?;

        assert!(
            n <= buf.len(),
            "`tokio::io::AsyncWrite` contract violation by `{}`: \
            reported number of bytes written ({n})\
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
        envelope: <T::Write as TransportWrite>::Envelope,
        header: mtproto::PlainHeader,
        buffer: &'a mut unbite::DynBuf,
        message_id: i64,
    ) -> Single<'a, W, T> {
        mtproto::pack_plain(header, buffer, message_id);

        self.single_impl(buffer, envelope)
    }

    pub fn single<'a>(
        &'a mut self,
        envelope: <T::Write as TransportWrite>::Envelope,
        header: mtproto::EncryptedHeader,
        buffer: &'a mut unbite::DynBuf,
        padding: mtproto::EncryptedPadding,
        auth_key: &mtproto::AuthKey,
        message: mtproto::DecryptedMessage,
        msg: mtproto::Msg,
    ) -> Single<'a, W, T> {
        mtproto::pack_encrypted(header, buffer, padding, auth_key, message, msg);

        self.single_impl(buffer, envelope)
    }

    fn single_impl<'a>(
        &'a mut self,
        buffer: &'a mut unbite::DynBuf,
        envelope: <T::Write as TransportWrite>::Envelope,
    ) -> Single<'a, W, T> {
        self.transport.pack(buffer, envelope);

        Single {
            writer: self,
            buffer,
            pos: 0,
        }
    }
}

pub struct Single<'a, W: WriterDriver, T: Transport> {
    writer: &'a mut Writer<W, T>,
    buffer: &'a mut unbite::DynBuf,
    pos: usize,
}

impl<'a, W: WriterDriver, T: Transport> Single<'a, W, T> {
    #[inline]
    pub fn pos(self) -> usize {
        self.pos
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), WriterError>> {
        loop {
            let buf = &self.buffer.as_slice()[self.pos..];

            if buf.is_empty() {
                return Poll::Ready(Ok(()));
            }

            let n = ready!(self.writer.poll_checked(cx, buf))?;

            self.pos += n.get();
        }
    }
}
