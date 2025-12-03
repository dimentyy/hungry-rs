mod queued;

use bytes::BytesMut;
use std::io;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

use crate::transport::{Transport, TransportWrite};

use crate::utils::ready_ok;
use crate::{Envelope, EnvelopeSize};
pub use queued::QueuedWriter;

pub struct Writer<W: AsyncWrite + Unpin, T: Transport> {
    driver: W,
    transport: T::Write,
}

impl<W: AsyncWrite + Unpin, T: Transport> Writer<W, T> {
    pub(crate) fn new(driver: W, transport: T::Write) -> Self {
        Self { driver, transport }
    }

    pub(crate) fn write<'a>(
        &'a mut self,
        buffer: &'a mut BytesMut,
        envelope: Envelope<T>,
    ) -> Write<'a, W, T> {
        let range = self.transport.pack(buffer, envelope);

        Write {
            writer: self,
            buffer,
            pos: range.start,
        }
    }
}

pub struct Write<'a, W: AsyncWrite + Unpin, T: Transport> {
    writer: &'a mut Writer<W, T>,
    buffer: &'a mut BytesMut,
    pos: usize,
}

impl<'a, W: AsyncWrite + Unpin, T: Transport> Write<'a, W, T> {
    #[inline]
    pub fn pos(self) -> usize {
        self.pos
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        loop {
            let buf = &self.buffer[self.pos..];

            if buf.is_empty() {
                crate::utils::dump(self.buffer.as_ref(), "WROTE");
                return Poll::Ready(Ok(()));
            }

            let n = ready_ok!(pin!(&mut self.writer.driver).poll_write(cx, buf));

            if n == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "wrote 0 bytes",
                )));
            }

            self.pos += n;
        }
    }
}

impl<'a, W: AsyncWrite + Unpin, T: Transport> Future for Write<'a, W, T> {
    type Output = io::Result<()>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
