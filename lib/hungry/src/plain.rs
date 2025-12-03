use std::fmt::Display;
use std::io;

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::mtproto::{EncryptedMessage, PlainEnvelope};
use crate::reader::{Error as ReaderError, Reader, ReaderBehaviour, Split};
use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::writer::Writer;
use crate::{mtproto, tl, Envelope};

#[derive(Debug, thiserror::Error)]
#[error("error")]
pub enum Error {
    Reader(#[from] ReaderError),
    Writer(io::Error),
    Deserialization(#[from] tl::de::Error),
    QuickAck(QuickAck),
    Encrypted(EncryptedMessage),
}

pub async fn send<
    T: Transport,
    R: AsyncRead + Unpin,
    B: ReaderBehaviour<Unpack = <Split as ReaderBehaviour>::Unpack>,
    W: AsyncWrite + Unpin,
    F: tl::Function,
>(
    r: &mut Reader<R, B, T>,
    w: &mut Writer<W, T>,
    func: &F,
    buffer: &mut BytesMut,
    transport: Envelope<T>,
    mtp: PlainEnvelope,
    message_id: i64,
) -> Result<F::Response, Error> {
    unsafe {
        buffer.set_len(func.serialized_len());
        func.serialize_unchecked(buffer.as_mut_ptr());
    }

    mtproto::pack::plain(buffer, mtp, message_id);

    w.write(buffer, transport).await.map_err(Error::Writer)?;

    let (buffer, unpack) = r.await.map_err(Error::Reader)?;

    let (data, next) = match unpack {
        Unpack::Packet(Packet { data, next }) => (data, next),
        Unpack::QuickAck(quick_ack) => {
            return Err(Error::QuickAck(quick_ack));
        }
    };

    let message = match mtproto::Message::unpack(&buffer[data.clone()]) {
        mtproto::Message::Plain(message) => message,
        mtproto::Message::Encrypted(message) => {
            return Err(Error::Encrypted(message));
        }
    };

    let mut buf = tl::de::Buf::new(&buffer[data.start + 20..data.end]);
    let value = tl::de::Deserialize::deserialize_checked(&mut buf)?;

    Ok(value)
}
