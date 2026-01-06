mod error;

use std::future::poll_fn;

use crate::reader::{Reader, ReaderDriver, ReaderResult};
use crate::transport::{Transport, Unpack};
use crate::writer::{Writer, WriterDriver};
use crate::{mtproto, tl};

pub use error::PlainError;

#[must_use]
pub struct Plain<T: Transport, R: ReaderDriver, W: WriterDriver> {
    reader: Reader<R, T>,
    writer: Writer<W, T>,
}

impl<T: Transport, R: ReaderDriver, W: WriterDriver> Plain<T, R, W> {
    #[inline]
    pub fn new(reader: Reader<R, T>, writer: Writer<W, T>) -> Self {
        Self { reader, writer }
    }

    pub async fn send<F: tl::Function>(
        &mut self,
        buffer: &mut unbite::DynBuf,
        f: &F,
    ) -> Result<F::Response, PlainError> {
        buffer.clear();

        let envelope = T::envelope(buffer);

        let header = buffer.split_raw_front();

        buffer.init_with(|spare_capacity| {
            let mut buf = tl::ser::Buf::uninit(spare_capacity);

            buf.ser(&F::CONSTRUCTOR_ID);
            buf.ser(f);

            buf.as_slice()
        });

        let mut fut = self.writer.single_plain(envelope, header, buffer, 0);

        poll_fn(|cx| fut.poll(cx)).await?;

        let unpack = match poll_fn(|cx| self.reader.poll(cx)).await {
            ReaderResult::Reserve(_) => todo!(),
            ReaderResult::Unpack(unpack) => unpack,
            ReaderResult::Error(err) => return Err(err.into()),
        };

        let data = match unpack {
            Unpack::Packet(packet) => packet.data,
            Unpack::QuickAck(_) => unimplemented!(),
        };

        let buf = &self.reader.buffer().as_slice()[data.clone()];

        let message = match mtproto::Message::unpack(buf) {
            mtproto::Message::Plain(message) => message,
            mtproto::Message::Encrypted(message) => {
                return Err(PlainError::EncryptedMessage(message));
            }
        };
        // TODO: msg_id, length check

        let mut buf = tl::de::Buf::new(&buf[mtproto::PlainMessage::HEADER_LEN..]);

        Ok(buf.de()?)
    }
}
