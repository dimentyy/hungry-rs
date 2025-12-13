mod dump;
mod error;
mod reserve;
mod split;

use bytes::BytesMut;
use tokio::io::{AsyncRead, ReadBuf};

use std::io;
use std::ops::ControlFlow;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use crate::transport::{Transport, TransportRead, Unpack};
use crate::utils::{BytesMutExt, ready_ok};

pub use dump::Dump;
pub use error::Error;
pub use reserve::Reserve;
pub use split::Split;

pub trait HandleBuffer {
    fn required(&mut self, buffer: &mut BytesMut, length: usize);
}

pub trait HandleOutput {
    type Output;

    fn acquired(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Output;
}

pub trait Handle: HandleBuffer + HandleOutput + Unpin {}
impl<T: HandleBuffer + HandleOutput + Unpin> Handle for T {}

pub struct Parted<B: HandleBuffer + Unpin, O: HandleOutput + Unpin> {
    pub buffer: B,
    pub output: O,
}

impl<B: HandleBuffer + Unpin, O: HandleOutput + Unpin> HandleBuffer for Parted<B, O> {
    fn required(&mut self, buffer: &mut BytesMut, length: usize) {
        self.buffer.required(buffer, length);
    }
}

impl<B: HandleBuffer + Unpin, O: HandleOutput + Unpin> HandleOutput for Parted<B, O> {
    type Output = O::Output;

    fn acquired(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Output {
        self.output.acquired(buffer, unpack)
    }
}

pub struct Reader<R: AsyncRead + Unpin, H: Handle, T: Transport> {
    driver: R,
    handle: H,
    transport: T::Read,
    buffer: BytesMut,
    length: usize,
}

impl<R: AsyncRead + Unpin, H: Handle, T: Transport> Reader<R, H, T> {
    pub(crate) fn new(driver: R, handle: H, transport: T::Read, mut buffer: BytesMut) -> Self {
        buffer.set_zero_len();

        Self {
            driver,
            handle,
            transport,
            buffer,
            length: T::Read::DEFAULT_BUF_LEN,
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        loop {
            if self.buffer.capacity() < self.length {
                self.handle.required(&mut self.buffer, 0);
                assert!(self.buffer.capacity() >= self.length);
            }

            ready_ok!(self.poll_read(cx, self.length));

            let unpack = match self.transport.unpack(self.buffer.as_mut()) {
                ControlFlow::Continue(length) => {
                    assert!(length > self.length);

                    self.length = length;

                    continue;
                }
                ControlFlow::Break(Err(err)) => {
                    self.buffer.clear();

                    self.length = T::Read::DEFAULT_BUF_LEN;

                    return Poll::Ready(Err(Error::Transport(err)));
                }
                ControlFlow::Break(Ok(unpack)) => unpack,
            };

            self.length = T::Read::DEFAULT_BUF_LEN;

            let output = self.handle.acquired(&mut self.buffer, unpack);

            assert!(self.buffer.is_empty());

            return Poll::Ready(Ok(output));
        }
    }

    fn poll_read(&mut self, cx: &mut Context<'_>, length: usize) -> Poll<io::Result<()>> {
        assert!(length <= self.buffer.capacity());

        if self.buffer.len() >= length {
            assert_eq!(self.buffer.len(), length);

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

            // SAFETY: data is initialized up to additional `n` bytes.
            unsafe { self.buffer.set_len(self.buffer.len() + n) };

            if n >= len {
                assert_eq!(n, len);

                return Poll::Ready(Ok(()));
            }
        }
    }
}

impl<R: AsyncRead + Unpin, H: Handle, T: Transport> Future for Reader<R, H, T> {
    type Output = Result<H::Output, Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
