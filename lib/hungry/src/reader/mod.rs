mod dump;
mod error;
mod reserve;
mod split;

use bytes::BytesMut;
use tokio::io::{AsyncRead, ReadBuf};

use std::io;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};

use crate::transport::{Error as TransportError, Transport, TransportRead, Unpack};
use crate::utils::{ready_ok, BytesMutExt};

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
            None => {
                if self.buffer.capacity() < T::HEADER {
                    self.handle.required(&mut self.buffer, T::HEADER);
                    assert!(self.buffer.capacity() >= T::HEADER);
                }

                assert!(self.buffer.len() < T::HEADER);

                let length = ready_ok!(self.poll_header(cx));
                self.length = Some(length);
                length
            }
        };

        if self.buffer.capacity() < length {
            self.handle.required(&mut self.buffer, length);
            assert!(self.buffer.capacity() >= length);
        }

        let unpack = match ready_ok!(self.poll_unpack(cx, length)) {
            Ok(unpack) => unpack,
            Err(err) => {
                self.length = None;
                self.buffer.clear();
                return Poll::Ready(Err(err.into()));
            }
        };

        self.length = None;

        let output = self.handle.acquired(&mut self.buffer, unpack);

        assert!(self.buffer.is_empty());

        Poll::Ready(Ok(output))
    }

    fn poll_header(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        // FIXME
        ready_ok!(self.poll_read(cx, 4));

        let required = self.transport.length(self.buffer.as_mut());

        Poll::Ready(Ok(required))
    }

    fn poll_unpack(
        &mut self,
        cx: &mut Context<'_>,
        length: usize,
    ) -> Poll<io::Result<Result<Unpack, TransportError>>> {
        ready_ok!(self.poll_read(cx, length));

        let unpacked = self.transport.unpack(self.buffer.as_mut());

        Poll::Ready(Ok(unpacked))
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

            let filled = buf.filled().len();

            if filled == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "read 0 bytes",
                )));
            }

            unsafe { self.buffer.set_len(self.buffer.len() + filled) };

            if filled >= len {
                assert_eq!(filled, len);

                return Poll::Ready(Ok(()));
            }
        }
    }
}

impl<R: AsyncRead + Unpin, H: Handle, T: Transport> Future for Reader<R, H, T> {
    type Output = Result<H::Output, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
