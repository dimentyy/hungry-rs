use std::fmt;
use std::future::poll_fn;
use std::ops::ControlFlow;

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::reader::{Reader, ReaderError};
use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::utils::BytesMutExt;
use crate::writer::{Writer, WriterError};
use crate::{Envelope, mtproto, tl};

use tl::ser::SerializeInto;

#[derive(Debug)]
pub enum Error {
    Reader(ReaderError),
    Writer(WriterError),
    QuickAck(QuickAck),
    EncryptedMessage(mtproto::EncryptedMessage),
    Deserialization {
        source: tl::de::Error,
        buffer: BytesMut,
    },
}

impl From<ReaderError> for Error {
    fn from(value: ReaderError) -> Self {
        Self::Reader(value)
    }
}

impl From<WriterError> for Error {
    fn from(value: WriterError) -> Self {
        Self::Writer(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Reader(err) => err.fmt(f),
            Error::Writer(err) => err.fmt(f),
            Error::QuickAck(_) => todo!(),
            Error::EncryptedMessage(_) => todo!(),
            Error::Deserialization { source, buffer: _ } => source.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

pub async fn send<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin, F: tl::Function>(
    reader: &mut Reader<R, T>,
    writer: &mut Writer<W, T>,
    func: &F,
    buffer: &mut BytesMut,
    transport: Envelope<T>,
    mtp: mtproto::PlainEnvelope,
    message_id: i64,
) -> Result<(i64, F::Response), Error> {
    assert!(buffer.spare_capacity_len() >= func.serialized_len() + 4);

    buffer.ser(&F::CONSTRUCTOR_ID);
    buffer.ser(func);

    let mut w = writer.single_plain(transport, mtp, buffer, message_id);

    poll_fn(|cx| w.poll(cx)).await.map_err(Error::Writer)?;

    let r = poll_fn(|cx| reader.poll(cx)).await;

    let ControlFlow::Continue(unpack) = r else {
        unimplemented!()
    };

    let data = match unpack? {
        Unpack::Packet(Packet { data }) => data,
        Unpack::QuickAck(quick_ack) => {
            return Err(Error::QuickAck(quick_ack));
        }
    };

    let buffer = reader.buffer().split();

    let message = match mtproto::Message::unpack(&buffer[data.clone()]) {
        mtproto::Message::Plain(message) => message,
        mtproto::Message::Encrypted(message) => {
            return Err(Error::EncryptedMessage(message));
        }
    };

    let buf = &buffer[data.start + mtproto::PlainMessage::HEADER_LEN..data.end];

    let response = match tl::de(buf) {
        Ok(response) => response,
        Err(source) => return Err(Error::Deserialization { source, buffer }),
    };

    Ok((message.message_id, response))
}
