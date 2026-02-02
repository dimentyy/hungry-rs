use std::collections::VecDeque;
use std::io;
use std::task::{Context, Poll};

use tokio::io::AsyncWrite;

use crate::transport::{Transport, TransportWrite};
use crate::writer::{Writer, WriterError};
use crate::{common, mtproto};

use common::infallible;

pub struct QueuedWriter<W: AsyncWrite + Unpin, T: Transport> {
    error: Option<io::Error>,
    driver: Writer<W, T>,
    buffers: VecDeque<unbite::DynBuf>,
}

impl<W: AsyncWrite + Unpin, T: Transport> QueuedWriter<W, T> {
    #[must_use]
    pub const fn new(driver: Writer<W, T>) -> Self {
        Self {
            error: None,
            driver,
            buffers: VecDeque::new(),
        }
    }

    #[must_use]
    pub(crate) fn with_buffer(driver: Writer<W, T>, buffer: unbite::DynBuf) -> Self {
        let mut buffers = VecDeque::with_capacity(1);
        buffers.push_back(buffer);

        Self {
            error: None,
            driver,
            buffers,
        }
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty() && self.error.is_none()
    }

    #[inline]
    #[must_use]
    pub fn vec_deque_capacity(&self) -> usize {
        self.buffers.capacity()
    }

    #[inline]
    pub fn shrink_vec_deque_to_fit(&mut self) {
        self.buffers.shrink_to_fit();
    }

    #[inline]
    pub fn shrink_vec_deque_to(&mut self, min_capacity: usize) {
        self.buffers.shrink_to(min_capacity);
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
            && !back.has_spare_capacity()
        {
            back.unsplit_back(buffer);
        } else {
            self.buffers.push_back(buffer);
        }

        ret
    }

    #[inline]
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

    #[inline]
    #[must_use]
    pub fn queue(
        &mut self,
        transport: T::Envelope,
        encrypted: mtproto::EncryptedEnvelope,
        mut buffer: unbite::DynBuf,
        auth_key: &mtproto::AuthKey,
        internal: mtproto::InternalHeader,
        msg: mtproto::Msg,
    ) -> Option<unbite::DynRaw> {
        encrypted.pack(&mut buffer, auth_key, internal, msg);

        self.queue_impl(buffer, transport)
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<unbite::DynBuf, WriterError>> {
        if let Some(err) = self.error.take() {
            return Poll::Ready(Err(WriterError::Io(err)));
        }

        let Some(buffer) = self.buffers.front_mut() else {
            return Poll::Pending;
        };

        let mut pos = 0;

        loop {
            let buf = &buffer.as_slice()[pos..];

            let Poll::Ready(ready) = self.driver.poll_checked(cx, buf) else {
                return if pos == 0 {
                    Poll::Pending
                } else {
                    Poll::Ready(Ok(buffer.split_to(pos)))
                };
            };

            let n = match ready {
                Ok(n) => n.get(),
                Err(err) if pos == 0 => {
                    return Poll::Ready(Err(WriterError::Io(err)));
                }
                Err(err) => {
                    self.error = Some(err);

                    return Poll::Ready(Ok(buffer.split_to(pos)));
                }
            };

            pos += n;

            if pos < buffer.len() {
                continue;
            }

            infallible! {
                let buffer = self.buffers.pop_front().unwrap();
            }

            return Poll::Ready(Ok(buffer));
        }
    }
}
