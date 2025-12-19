mod queued;

use std::io;
use std::num::NonZeroUsize;
use std::pin::pin;
use std::task::{Context, Poll};

use bytes::BytesMut;
use tokio::io::AsyncWrite;

use crate::transport::{Transport, TransportWrite};
use crate::utils::ready_ok;
use crate::{Envelope, mtproto};

pub use queued::QueuedWriter;

pub trait WriterDriver: AsyncWrite + Unpin {}
impl<T: AsyncWrite + Unpin> WriterDriver for T {}

pub struct Writer<W: WriterDriver, T: Transport> {
    driver: W,
    transport: T::Write,
}

impl<W: WriterDriver, T: Transport> Writer<W, T> {
    pub(crate) fn new(driver: W, transport: T::Write) -> Self {
        Self { driver, transport }
    }

    pub fn driver(&mut self) -> &mut W {
        &mut self.driver
    }

    fn poll_checked(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<NonZeroUsize>> {
        let n = ready_ok!(pin!(&mut self.driver).poll_write(cx, buf));

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
        buffer: &'a mut BytesMut,
        transport: Envelope<T>,
        mtp: mtproto::PlainEnvelope,
        message_id: i64,
    ) -> Single<'a, W, T> {
        mtproto::pack_plain(buffer, mtp, message_id);

        self.single_impl(buffer, transport)
    }

    pub fn single<'a>(
        &'a mut self,
        buffer: &'a mut BytesMut,
        transport: Envelope<T>,
        mtp: mtproto::EncryptedEnvelope,
        auth_key: &mtproto::AuthKey,
        message: mtproto::DecryptedMessage,
        msg: mtproto::Msg,
    ) -> Single<'a, W, T> {
        mtproto::pack_encrypted(buffer, mtp, auth_key, message, msg);

        self.single_impl(buffer, transport)
    }

    fn single_impl<'a>(
        &'a mut self,
        buffer: &'a mut BytesMut,
        transport: Envelope<T>,
    ) -> Single<'a, W, T> {
        let range = self.transport.pack(buffer, transport);

        Single {
            writer: self,
            buffer,
            pos: range.start,
        }
    }
}

pub struct Single<'a, W: WriterDriver, T: Transport> {
    writer: &'a mut Writer<W, T>,
    buffer: &'a mut BytesMut,
    pos: usize,
}

impl<'a, W: WriterDriver, T: Transport> Single<'a, W, T> {
    #[inline]
    pub fn pos(self) -> usize {
        self.pos
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        loop {
            let buf = &self.buffer[self.pos..];

            if buf.is_empty() {
                return Poll::Ready(Ok(()));
            }

            let n = ready_ok!(self.writer.poll_checked(cx, buf));

            self.pos += n.get();
        }
    }
}
