use std::future::poll_fn;

use crate::reader::{Reader, ReaderDriver, ReaderError, ReaderResult};
use crate::transport::{Transport, Unpack};
use crate::writer::{Writer, WriterDriver, WriterError};
use crate::{mtproto, tl};

#[must_use]
#[derive(Debug)]
pub enum PlainError {
    Reader(ReaderError),
    Writer(WriterError),
    EncryptedMessage(mtproto::EncryptedMessage),
}

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

    #[must_use]
    pub async fn send<F: tl::Function>(
        &mut self,
        buffer: &mut unbite::DynBuf,
        f: &F,
    ) -> F::Response {
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

        poll_fn(|cx| fut.poll(cx)).await.expect("todo");

        let unpack = match poll_fn(|cx| self.reader.poll(cx)).await {
            ReaderResult::Reserve(_) => todo!(),
            ReaderResult::Unpack(unpack) => unpack,
            ReaderResult::Error(err) => todo!(),
        };

        let data = match unpack {
            Unpack::Packet(packet) => packet.data,
            Unpack::QuickAck(_) => todo!(),
        };

        let _ = match mtproto::Message::unpack(&self.reader.buffer().as_slice()[data.clone()]) {
            mtproto::Message::Plain(message) => message,
            mtproto::Message::Encrypted(_) => todo!(),
        };

        let data = data.start + mtproto::PlainMessage::HEADER_LEN..data.end;

        let mut buf = tl::de::Buf::new(&self.reader.buffer().as_slice()[data]);

        buf.de().expect("todo")
    }
}
