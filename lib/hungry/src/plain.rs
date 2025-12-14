use std::{fmt, io};

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::utils::BytesMutExt;
use crate::{mtproto, reader, tl, writer, Envelope};

use tl::de::{Buf, Deserialize};
use tl::ser::SerializeInto;

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
            Error::Deserialization { source, buffer: _ } => source.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

pub async fn send<
    T: Transport,
    R: AsyncRead + Unpin,
    H: reader::HandleReader<Output = <reader::Split as reader::ProcessReaderPacket>::Output>,
    W: AsyncWrite + Unpin,
    F: tl::Function,
>(
    r: &mut reader::Reader<R, T, H>,
    w: &mut writer::Writer<W, T>,
    func: &F,
    buffer: &mut BytesMut,
    transport: Envelope<T>,
    mtp: mtproto::PlainEnvelope,
    message_id: i64,
) -> Result<(i64, F::Response), Error> {
    assert!(buffer.spare_capacity_len() >= func.serialized_len());

    buffer.ser(func);

    w.single_plain(buffer, transport, mtp, message_id)
        .await
        .map_err(Error::Writer)?;

    let (buffer, unpack) = r.await?;

    let data = match unpack {
        Unpack::Packet(Packet { data }) => data,
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

    let buf = &buffer[data.start + mtproto::PlainMessage::HEADER_LEN..data.end];

    let response = match F::Response::deserialize(&mut Buf::new(buf)) {
        Ok(response) => response,
        Err(source) => return Err(Error::Deserialization { source, buffer }),
    };

    Ok((message.message_id, response))
}
