#![forbid(clippy::todo)]

mod error;

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, ReadBuf};

use crate::transport::{Packet, Transport, TransportRead, Unpack, UnpackResult};

pub use error::ReaderError;

const BUFFER_NO_ROTATE_THRESHOLD: usize = 16 * 1024;

#[must_use]
#[derive(Debug)]
pub enum ReaderResult {
    Reserve(usize),
    Unpack(Unpack),
    Error(ReaderError),
}

pub struct Reader<R: AsyncRead + Unpin, T: Transport> {
    driver: R,
    transport: T::Read,
    buffer: unbite::DynBuf,
    offset: usize,
}

impl<R: AsyncRead + Unpin, T: Transport> Reader<R, T> {
    pub(crate) fn new(driver: R, transport: T::Read, buffer: unbite::DynBuf) -> Self {
        Self {
            driver,
            transport,
            buffer,
            offset: 0,
        }
    }

    #[inline]
    pub fn as_mut_slice(&mut self, packet: Packet) -> &mut [u8] {
        &mut self.buffer.as_mut_slice()[packet.data]
    }

    fn rotate_buffer(&mut self, packet_len: usize) {
        if self.offset == 0 {
            return;
        }

        let buffer_len = self.buffer.len() - self.offset;
        let capacity = buffer_len + self.buffer.spare_capacity_len();

        if buffer_len < BUFFER_NO_ROTATE_THRESHOLD || packet_len > capacity {
            self.buffer.as_mut_slice().copy_within(self.offset.., 0);
            self.buffer.truncate(self.buffer.len() - self.offset);
            self.offset = 0;
        }
    }

    /// # Panics
    ///
    /// * If the inner `buffer` length was truncated externally.
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ReaderResult> {
        assert!(self.offset <= self.buffer.len());

        if self.offset == self.buffer.len() {
            self.offset = 0;
            self.buffer.clear();
        }

        'unpack: loop {
            let buffer = &mut self.buffer.as_mut_slice()[self.offset..];

            let length = match self.transport.unpack(buffer) {
                UnpackResult::Unpacked { result, offset } => {
                    self.offset += offset;

                    return Poll::Ready(match result {
                        // Quick ACK is passed through.
                        Ok(unpack) => ReaderResult::Unpack(unpack),
                        Err(err) => ReaderResult::Error(ReaderError::Transport(err)),
                    });
                }
                UnpackResult::Continue { length } => length,
            };

            if length > self.buffer.capacity() {
                return Poll::Ready(ReaderResult::Reserve(length));
            }

            self.rotate_buffer(length);

            loop {
                if let Err(err) = ready!(self.poll_checked(cx)) {
                    return Poll::Ready(ReaderResult::Error(ReaderError::Io(err)));
                }

                if self.buffer.len() >= self.offset + length {
                    continue 'unpack;
                }
            }
        }
    }

    fn poll_checked(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let spare_capacity_len = self.buffer.spare_capacity_len();
        let mut buf = ReadBuf::uninit(self.buffer.spare_capacity_mut());

        ready!(Pin::new(&mut self.driver).poll_read(cx, &mut buf))?;

        let n = buf.filled().len();

        if n == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::ConnectionReset,
                "read 0 bytes",
            )));
        }

        assert!(
            n <= spare_capacity_len,
            "`tokio::io::AsyncRead` contract violation by `{}`: \
            reported number of bytes read ({n}) \
            exceeds the buffer spare capacity length ({spare_capacity_len})",
            std::any::type_name::<R>(),
        );

        // SAFETY: we have read `n` bytes.
        unsafe { self.buffer.advance(n) };

        Poll::Ready(Ok(()))
    }
}
