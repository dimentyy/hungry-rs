mod dump;
mod error;
mod plain;
mod split;

use bytes::BytesMut;
use tokio::io::{AsyncRead, ReadBuf};

use std::io;
use std::pin::{pin, Pin};
use std::task::{ready, Context, Poll};

use crate::transport::{Transport, TransportRead, Unpack};
use crate::utils::{ready_ok, BytesMutExt};

pub use dump::Dump;
pub use error::Error;
pub use plain::{PlainDeserializer, PlainDeserializerError};
pub use split::Split;

pub trait ReaderBehaviour: Unpin {
    type Unpack;

    fn required(&mut self, buffer: &mut BytesMut, length: usize);
    fn acquired(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Unpack;
}

pub struct Reader<R: AsyncRead + Unpin, B: ReaderBehaviour, T: Transport> {
    driver: R,
    behaviour: B,
    transport: T::Read,
    buffer: BytesMut,
    length: Option<usize>,
}

impl<R: AsyncRead + Unpin, B: ReaderBehaviour, T: Transport> Reader<R, B, T> {
    pub(crate) fn new(driver: R, behaviour: B, transport: T::Read, mut buffer: BytesMut) -> Self {
        buffer.set_zero_len();

        Self {
            driver,
            transport,
            behaviour,
            buffer,
            length: None,
        }
    }

    pub(crate) fn switch_behaviour<S: ReaderBehaviour>(self, behaviour: S) -> (Reader<R, S, T>, B) {
        (
            Reader {
                driver: self.driver,
                behaviour,
                transport: self.transport,
                buffer: self.buffer,
                length: self.length,
            },
            self.behaviour,
        )
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<B::Unpack, Error>> {
        let length = match self.length {
            Some(length) => length,
            None => {
                if self.buffer.capacity() < T::HEADER {
                    self.behaviour.required(&mut self.buffer, T::HEADER);
                    assert!(self.buffer.capacity() >= T::HEADER);
                }

                assert!(self.buffer.len() < T::HEADER);

                let length = ready_ok!(self.poll_header(cx));
                self.length = Some(length);
                length
            }
        };

        if self.buffer.capacity() < length {
            self.behaviour.required(&mut self.buffer, length);
            assert!(self.buffer.capacity() >= length);
        }

        let unpack = ready_ok!(self.poll_unpack(cx, length));

        self.length = None;

        Poll::Ready(Ok(self.behaviour.acquired(&mut self.buffer, unpack)))
    }

    fn poll_header(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
        ready_ok!(self.poll_part_read(cx, T::HEADER));

        let required = self.transport.length(self.buffer.as_mut());

        Poll::Ready(Ok(required))
    }

    fn poll_unpack(&mut self, cx: &mut Context<'_>, length: usize) -> Poll<Result<Unpack, Error>> {
        ready_ok!(self.poll_part_read(cx, length));

        let unpacked = self.transport.unpack(self.buffer.as_mut());

        Poll::Ready(unpacked.map_err(Error::Unpack))
    }

    fn poll_part_read(&mut self, cx: &mut Context<'_>, length: usize) -> Poll<io::Result<()>> {
        assert!(length <= self.buffer.capacity());

        if self.buffer.len() >= length {
            assert_eq!(self.buffer.len(), length);

            return Poll::Ready(Ok(()));
        }

        loop {
            let len = length - self.buffer.len();
            let mut buf = ReadBuf::uninit(&mut self.buffer.spare_capacity_mut()[..len]);

            ready_ok!(dbg!(pin!(&mut self.driver).poll_read(cx, &mut buf)));

            let filled = buf.filled().len();

            unsafe { self.buffer.set_len(self.buffer.len() + filled) };

            if filled >= len {
                assert_eq!(filled, len);

                return Poll::Ready(Ok(()));
            }

            if filled == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::ConnectionReset,
                    "read 0 bytes",
                )));
            }
        }
    }
}

impl<R: AsyncRead + Unpin, B: ReaderBehaviour, T: Transport> Future for Reader<R, B, T> {
    type Output = Result<B::Unpack, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
