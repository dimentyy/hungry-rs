use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::BytesMut;
use tokio::io::AsyncWrite;

use crate::transport::{Transport, TransportWrite};
use crate::{Envelope, mtproto, writer};

pub struct QueuedWriter<W: AsyncWrite + Unpin, T: Transport> {
    error: Option<io::Error>,
    driver: writer::Writer<W, T>,
    buffers: VecDeque<BytesMut>,
}

impl<W: AsyncWrite + Unpin, T: Transport> QueuedWriter<W, T> {
    #[must_use]
    pub fn new(driver: writer::Writer<W, T>) -> Self {
        Self {
            error: None,
            driver,
            buffers: VecDeque::new(),
        }
    }

    /// Returned buffer may be out-of-order due to multiple being queued at the same time.
    fn queue_impl(&mut self, mut buffer: BytesMut, envelope: Envelope<T>) -> Option<BytesMut> {
        let packed = self.driver.transport.pack(&mut buffer, envelope);

        let result = if packed.start > 0 {
            Some(buffer.split_to(packed.start))
        } else {
            None
        };

        self.buffers.push_back(buffer);

        result
    }

    #[must_use = "the `BytesMut` must be reused to avoid unnecessary memory reallocation"]
    pub fn queue_plain(
        &mut self,
        mut buffer: BytesMut,
        transport: Envelope<T>,
        mtp: mtproto::PlainEnvelope,
        message_id: i64,
    ) -> Option<BytesMut> {
        mtproto::pack_plain(&mut buffer, mtp, message_id);

        self.queue_impl(buffer, transport)
    }

    #[must_use = "the `BytesMut` must be reused to avoid unnecessary memory reallocation"]
    pub fn queue(
        &mut self,
        mut buffer: BytesMut,
        transport: Envelope<T>,
        mtp: mtproto::Envelope,
        auth_key: &mtproto::AuthKey,
        message: mtproto::DecryptedMessage,
        msg: mtproto::Msg,
    ) -> Option<BytesMut> {
        mtproto::pack_encrypted(&mut buffer, mtp, auth_key, message, msg);

        self.queue_impl(buffer, transport)
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<BytesMut>> {
        if let Some(error) = self.error.take() {
            return Poll::Ready(Err(error));
        }

        let Some(buffer) = self.buffers.front_mut() else {
            return Poll::Pending;
        };

        let mut pos = 0;

        loop {
            let ready = match self.driver.poll_checked(cx, &buffer[pos..]) {
                Poll::Ready(ready) => ready,
                Poll::Pending if pos == 0 => return Poll::Pending,
                Poll::Pending => return Poll::Ready(Ok(buffer.split_to(pos))),
            };

            let n = match ready {
                Ok(n) => n.get(),
                Err(error) if pos == 0 => return Poll::Ready(Err(error)),
                Err(error) => {
                    self.error = Some(error);

                    return Poll::Ready(Ok(buffer.split_to(pos)));
                }
            };

            pos += n;

            if pos < buffer.len() {
                continue;
            }

            assert_eq!(pos, buffer.len());

            return Poll::Ready(Ok(self.buffers.pop_front().unwrap()));
        }
    }
}

impl<W: AsyncWrite + Unpin, T: Transport> Future for QueuedWriter<W, T> {
    type Output = io::Result<BytesMut>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
