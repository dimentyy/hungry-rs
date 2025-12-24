use std::collections::VecDeque;
use std::io;
use std::task::{Context, Poll};

use bytes::BytesMut;
use tokio::io::AsyncWrite;

use crate::transport::{Transport, TransportWrite};
use crate::utils::BytesMutExt;
use crate::writer::{Writer, WriterError};
use crate::{Envelope, mtproto};

pub struct QueuedWriter<W: AsyncWrite + Unpin, T: Transport> {
    error: Option<io::Error>,
    driver: Writer<W, T>,
    buffers: VecDeque<BytesMut>,
}

impl<W: AsyncWrite + Unpin, T: Transport> QueuedWriter<W, T> {
    #[must_use]
    pub fn new(driver: Writer<W, T>) -> Self {
        Self {
            error: None,
            driver,
            buffers: VecDeque::new(),
        }
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty() || self.error.is_some()
    }

    /// Returned buffer may be out-of-order due to multiple being queued at the same time.
    fn queue_impl(
        &mut self,
        mut buffer: BytesMut,
        envelope: Envelope<T>,
    ) -> (Option<BytesMut>, Option<BytesMut>) {
        let packed = self.driver.transport.pack(&mut buffer, envelope);

        let header = if packed.start > 0 {
            Some(buffer.split_to(packed.start))
        } else {
            None
        };

        let footer = if buffer.has_spare_capacity() {
            Some(buffer.split_off(buffer.len()))
        } else {
            None
        };

        if let Some(back) = self.buffers.back_mut()
            && back.can_unsplit(&buffer)
        {
            back.unsplit(buffer);
        } else {
            self.buffers.push_back(buffer);
        }

        (header, footer)
    }

    #[must_use = "the `BytesMut` must be reused to avoid unnecessary memory reallocation"]
    pub fn queue_plain(
        &mut self,
        transport: Envelope<T>,
        mtp: mtproto::PlainEnvelope,
        mut buffer: BytesMut,
        message_id: i64,
    ) -> (Option<BytesMut>, Option<BytesMut>) {
        mtproto::pack_plain(mtp, &mut buffer, message_id);

        self.queue_impl(buffer, transport)
    }

    #[must_use = "the `BytesMut` must be reused to avoid unnecessary memory reallocation"]
    pub fn queue(
        &mut self,
        transport: Envelope<T>,
        mtp: mtproto::EncryptedEnvelope,
        mut buffer: BytesMut,
        auth_key: &mtproto::AuthKey,
        message: mtproto::DecryptedMessage,
        msg: mtproto::Msg,
    ) -> (Option<BytesMut>, Option<BytesMut>) {
        mtproto::pack_encrypted(mtp, &mut buffer, auth_key, message, msg);

        self.queue_impl(buffer, transport)
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<BytesMut, WriterError>> {
        if let Some(error) = self.error.take() {
            return Poll::Ready(Err(WriterError::Io(error)));
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
                Err(err) if pos == 0 => return Poll::Ready(Err(WriterError::Io(err))),
                Err(err) => {
                    // Immediately wake the task so the error will be returned.
                    cx.waker().wake_by_ref();

                    self.error = Some(err);

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
