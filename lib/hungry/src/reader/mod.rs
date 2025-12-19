mod error;

use std::io;
use std::ops::ControlFlow;
use std::pin::pin;
use std::task::{Context, Poll, ready};

use bytes::BytesMut;
use tokio::io::{AsyncRead, ReadBuf};

use crate::transport::{Transport, TransportRead, Unpack};
use crate::utils::ready_ok;

pub use error::Error;

pub trait ReaderDriver: AsyncRead + Unpin {}
impl<T: AsyncRead + Unpin> ReaderDriver for T {}

pub struct Reader<R: ReaderDriver, T: Transport> {
    driver: R,
    transport: T::Read,
    buffer: BytesMut,
    pos: usize,
    end: usize,
}

impl<R: ReaderDriver, T: Transport> Reader<R, T> {
    pub(crate) fn new(driver: R, transport: T::Read, buffer: BytesMut) -> Self {
        assert!(buffer.is_empty());

        Self {
            driver,
            transport,
            buffer,
            pos: 0,
            end: T::Read::DEFAULT_BUF_LEN,
        }
    }

    pub fn buffer(&mut self) -> &mut BytesMut {
        &mut self.buffer
    }

    fn reset(&mut self) {
        self.pos = 0;

        self.end = T::Read::DEFAULT_BUF_LEN;
    }

    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<ControlFlow<usize, Result<Unpack, Error>>> {
        assert_eq!(
            self.buffer.len(),
            self.pos,
            "buffer length have been modified externally",
        );

        loop {
            if self.buffer.capacity() < self.end {
                return Poll::Ready(ControlFlow::Break(self.end));
            }

            if let Err(err) = ready!(self.poll_read(cx, self.end)) {
                return Poll::Ready(ControlFlow::Continue(Err(Error::Io(err))));
            }

            let unpack = match self.transport.unpack(self.buffer.as_mut()) {
                ControlFlow::Continue(length) => {
                    assert!(length > self.end);

                    self.end = length;

                    continue;
                }
                ControlFlow::Break(Err(err)) => {
                    self.reset();

                    self.buffer.clear();

                    return Poll::Ready(ControlFlow::Continue(Err(Error::Transport(err))));
                }
                ControlFlow::Break(Ok(unpack)) => unpack,
            };

            self.reset();

            return Poll::Ready(ControlFlow::Continue(Ok(unpack)));
        }
    }

    fn poll_read(&mut self, cx: &mut Context<'_>, length: usize) -> Poll<io::Result<()>> {
        assert!(length <= self.buffer.capacity());

        if self.buffer.len() >= length {
            return Poll::Ready(Ok(()));
        }

        loop {
            let len = length - self.buffer.len();
            let mut buf = ReadBuf::uninit(&mut self.buffer.spare_capacity_mut()[..len]);

            ready_ok!(pin!(&mut self.driver).poll_read(cx, &mut buf));

            let n = buf.filled().len();

            if n == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "read 0 bytes",
                )));
            }

            assert!(
                n <= len,
                "`tokio::io::AsyncRead` contract violation by `{}`: \
                reported number of bytes read ({n}) \
                exceeds the buffer length ({len})",
                std::any::type_name::<R>(),
            );

            self.pos += n;

            // SAFETY: data is initialized up to `self.pos` bytes.
            unsafe { self.buffer.set_len(self.pos) };

            if n < len {
                continue;
            }

            return Poll::Ready(Ok(()));
        }
    }
}
