use std::collections::VecDeque;
use std::io;
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;

use crate::transport::{Transport, TransportWrite};
use crate::writer::{Writer, WriterError};
use crate::{mtproto};

pub struct QueuedWriter<W: AsyncWrite + Unpin, T: Transport> {
    error: Option<io::Error>,
    driver: Writer<W, T>,
    buffers: VecDeque<unbite::DynBuf>,
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
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty() || self.error.is_some()
    }

    fn queue_impl(
        &mut self,
        mut buffer: unbite::DynBuf,
        envelope: T::Envelope,
    ) -> Option<unbite::DynRaw> {
        self.driver.transport.pack(&mut buffer, envelope);

        let ret = if buffer.has_spare_capacity() {
            Some(buffer.split())
        } else {
            None
        };

        // Only unsplit with the last buffer. All packets are strictly ordered.
        if let Some(back) = self.buffers.back_mut()
            && back.can_unsplit_dyn_buf_back(&buffer)
        {
            back.unsplit_back(buffer);
        } else {
            self.buffers.push_back(buffer);
        }

        ret
    }

    #[must_use]
    pub fn queue_plain(
        &mut self,
        transport: T::Envelope,
        header: mtproto::PlainHeader,
        mut buffer: unbite::DynBuf,
        message_id: i64,
    ) -> Option<unbite::DynRaw> {
        mtproto::pack_plain(header, &mut buffer, message_id);

        self.queue_impl(buffer, transport)
    }

    #[must_use]
    pub fn queue(
        &mut self,
        transport: T::Envelope,
        header: mtproto::EncryptedHeader,
        mut buffer: unbite::DynBuf,
        padding: mtproto::EncryptedPadding,
        auth_key: &mtproto::AuthKey,
        message: mtproto::DecryptedMessage,
        msg: mtproto::Msg,
    ) -> Option<unbite::DynRaw> {
        mtproto::pack_encrypted(header, &mut buffer, padding, auth_key, message, msg);

        self.queue_impl(buffer, transport)
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<unbite::DynBuf, WriterError>> {
        if let Some(error) = self.error.take() {
            return Poll::Ready(Err(WriterError::Io(error)));
        }

        let Some(buffer) = self.buffers.front_mut() else {
            return Poll::Pending;
        };

        let mut pos = 0;

        loop {
            let ready = match self.driver.poll_checked(cx, &buffer.as_slice()[pos..]) {
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
            
            return Poll::Ready(Ok(self.buffers.pop_front().unwrap()));
        }
    }
}
