mod dump;
mod error;
mod reserve;
mod split;

use std::io;
use std::ops::ControlFlow;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use bytes::BytesMut;
use tokio::io::{AsyncRead, ReadBuf};

use crate::transport::{Transport, TransportRead, Unpack};
use crate::utils::ready_ok;

pub use dump::Dump;
pub use error::Error;
pub use reserve::Reserve;
pub use split::Split;

pub trait ReserveReaderBuffer {
    fn reserve(&mut self, buffer: &mut BytesMut, length: usize);
}

pub trait ProcessReaderPacket {
    type Output;

    fn process(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Output;
}

pub trait HandleReader: ReserveReaderBuffer + ProcessReaderPacket + Unpin {}
impl<T: ReserveReaderBuffer + ProcessReaderPacket + Unpin> HandleReader for T {}

pub struct Parted<R: ReserveReaderBuffer + Unpin, P: ProcessReaderPacket + Unpin> {
    pub reserve: R,
    pub process: P,
}

impl<R: ReserveReaderBuffer + Unpin, P: ProcessReaderPacket + Unpin> ReserveReaderBuffer
    for Parted<R, P>
{
    fn reserve(&mut self, buffer: &mut BytesMut, length: usize) {
        self.reserve.reserve(buffer, length);
    }
}

impl<R: ReserveReaderBuffer + Unpin, P: ProcessReaderPacket + Unpin> ProcessReaderPacket
    for Parted<R, P>
{
    type Output = P::Output;

    fn process(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Output {
        self.process.process(buffer, unpack)
    }
}

pub struct Reader<R: AsyncRead + Unpin, T: Transport, H: HandleReader> {
    driver: R,
    transport: T::Read,
    handle: H,
    buffer: BytesMut,
    length: usize,
}

impl<R: AsyncRead + Unpin, T: Transport, H: HandleReader> Reader<R, T, H> {
    pub(crate) fn new(driver: R, transport: T::Read, handle: H, buffer: BytesMut) -> Self {
        assert!(buffer.is_empty());

        Self {
            driver,
            transport,
            handle,
            buffer,
            length: T::Read::DEFAULT_BUF_LEN,
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        loop {
            if self.buffer.capacity() < self.length {
                self.handle.reserve(&mut self.buffer, 0);
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

            let output = self.handle.process(&mut self.buffer, unpack);

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

impl<R: AsyncRead + Unpin, T: Transport, H: HandleReader> Future for Reader<R, T, H> {
    type Output = Result<H::Output, Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
