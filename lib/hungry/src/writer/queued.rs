use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::BytesMut;
use tokio::io::AsyncWrite;

use crate::transport::{Transport, TransportWrite};
use crate::utils::ready_ok;
use crate::{mtproto, writer, Envelope};

pub struct QueuedWriter<W: AsyncWrite + Unpin, T: Transport> {
    driver: writer::Writer<W, T>,
    buffers: VecDeque<BytesMut>,
}

impl<W: AsyncWrite + Unpin, T: Transport> QueuedWriter<W, T> {
    #[must_use]
    pub fn new(driver: writer::Writer<W, T>) -> Self {
        Self {
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
        let Some(buffer) = self.buffers.front_mut() else {
            return Poll::Pending;
        };

        // Loop may not be used here because a written buffer will be lost due to an error.
        // Storing io::Error in the Writer to return in the next poll would be an overkill.
        let n = ready_ok!(self.driver.poll_checked(cx, buffer)).get();

        let buffer = if n >= buffer.len() {
            assert_eq!(n, buffer.len());

            self.buffers.pop_front().unwrap()
        } else {
            buffer.split_to(n)
        };

        Poll::Ready(Ok(buffer))
    }
}

impl<W: AsyncWrite + Unpin, T: Transport> Future for QueuedWriter<W, T> {
    type Output = io::Result<BytesMut>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
