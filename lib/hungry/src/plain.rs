use std::{fmt, io};

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::{mtproto, reader, tl, writer, Envelope};

#[derive(Debug)]
pub enum Error {
    Reader(reader::Error),
    Writer(io::Error),
    QuickAck(QuickAck),
    EncryptedMessage(mtproto::EncryptedMessage),
    Deserialization {
        source: tl::de::Error,
        buffer: BytesMut,
    },
}

impl From<reader::Error> for Error {
    fn from(value: reader::Error) -> Self {
        Self::Reader(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Reader(err) => err.fmt(f),
            Error::Writer(err) => err.fmt(f),
            Error::QuickAck(_) => todo!(),
            Error::EncryptedMessage(_) => todo!(),
            Error::Deserialization { source, buffer } => source.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

pub async fn send<
    T: Transport,
    R: AsyncRead + Unpin,
    H: reader::Handle<Output = <reader::Split as reader::HandleOutput>::Output>,
    W: AsyncWrite + Unpin,
    F: tl::Function,
>(
    r: &mut reader::Reader<R, H, T>,
    w: &mut writer::Writer<W, T>,
    func: &F,
    buffer: &mut BytesMut,
    transport: Envelope<T>,
    mtp: mtproto::PlainEnvelope,
    message_id: i64,
) -> Result<F::Response, Error> {
    unsafe {
        buffer.set_len(func.serialized_len());
        func.serialize_unchecked(buffer.as_mut_ptr());
    }

    w.single(buffer, transport, mtp, message_id)
        .await
        .map_err(Error::Writer)?;

    let (buffer, unpack) = r.await?;

    let (data, next) = match unpack {
        Unpack::Packet(Packet { data, next }) => (data, next),
        Unpack::QuickAck(quick_ack) => {
            return Err(Error::QuickAck(quick_ack));
        }
    };

    let message = match mtproto::Message::unpack(&buffer[data.clone()]) {
        mtproto::Message::Plain(message) => message,
        mtproto::Message::Encrypted(message) => {
            return Err(Error::EncryptedMessage(message));
        }
    };

    match tl::de::checked(&buffer[data.start + 20..data.end]) {
        Ok(response) => Ok(response),
        Err(source) => Err(Error::Deserialization { source, buffer }),
    }
}
