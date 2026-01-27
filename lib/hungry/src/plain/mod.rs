mod error;

use std::future::poll_fn;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::reader::{Reader, ReaderResult};
use crate::transport::{Transport, Unpack};
use crate::writer::Writer;
use crate::{mtproto, tl};

pub use error::PlainError;

#[must_use]
pub struct Plain<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: Writer<W, T>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Plain<T, R, W> {
    #[inline]
    pub const fn new(reader: Reader<R, T>, writer: Writer<W, T>) -> Self {
        Self { reader, writer }
    }

    #[inline]
    pub fn into_inner(self) -> (Reader<R, T>, Writer<W, T>) {
        (self.reader, self.writer)
    }

    #[expect(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn send<F: tl::Function + Sync>(
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

        let packet = match unpack {
            Unpack::Packet(packet) => packet,
            Unpack::QuickAck(_) => unimplemented!(),
        };

        let buf = self.reader.as_mut_slice(packet);

        if buf.len() < mtproto::PlainMsgHeader::LEN {
            todo!()
        }

        let (auth_key_id, buf) = buf.split_first_chunk().unwrap();

        if let Some(_auth_key_id) = mtproto::auth_key_id(*auth_key_id) {
            todo!()
        }

        let (header, buf) = buf.split_first_chunk().unwrap();

        let header = mtproto::PlainMsgHeader::unpack(*header);

        let data_length = header.message_data_length;

        let Ok(data_length) = usize::try_from(data_length) else {
            todo!()
        };

        if data_length != buf.len() {
            todo!()
        }

        if !data_length.is_multiple_of(4) {
            todo!()
        }

        let mut buf = tl::de::Buf::new(buf);

        Ok(buf.de()?)
    }
}
