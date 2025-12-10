mod dump;
mod error;
mod reserve;
mod split;

use bytes::BytesMut;
use tokio::io::{AsyncRead, ReadBuf};

use std::io;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};

use crate::transport::{Transport, TransportRead, Unpack};
use crate::utils::{ready_ok, BytesMutExt, SliceExt};

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
    length: Option<usize>,
}

impl<R: AsyncRead + Unpin, H: Handle, T: Transport> Reader<R, H, T> {
    pub(crate) fn new(driver: R, handle: H, transport: T::Read, mut buffer: BytesMut) -> Self {
        buffer.set_zero_len();

        Self {
            driver,
            handle,
            transport,
            buffer,
            length: None,
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        let length = match self.length {
            Some(length) => length,
            None => ready_ok!(self.poll_length(cx)),
        };

        self.poll_unpack(cx, length)
    }

    fn poll_length(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        if self.buffer.capacity() < 4 {
            self.handle.required(&mut self.buffer, 4);
            assert!(self.buffer.capacity() >= 4);
        }

        assert!(self.buffer.len() < 4);

        ready_ok!(self.poll_read(cx, 4));

        let length = self.transport.length(self.buffer.arr_mut());

        self.length = Some(length);

        Poll::Ready(Ok(length))
    }

    fn poll_unpack(
        &mut self,
        cx: &mut Context<'_>,
        length: usize,
    ) -> Poll<<Self as Future>::Output> {
        if self.buffer.capacity() < length {
            self.handle.required(&mut self.buffer, length);
            assert!(self.buffer.capacity() >= length);
        }

        ready_ok!(self.poll_read(cx, length));

        self.length = None;

        let unpack = match self.transport.unpack(self.buffer.as_mut()) {
            Ok(unpack) => unpack,
            Err(err) => {
                self.buffer.clear();

                return Poll::Ready(Err(Error::Transport(err)));
            }
        };

        let output = self.handle.acquired(&mut self.buffer, unpack);

        assert!(self.buffer.is_empty());

        Poll::Ready(Ok(output))
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
