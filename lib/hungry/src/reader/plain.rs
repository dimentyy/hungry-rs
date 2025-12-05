use std::marker::PhantomData;

use bytes::BytesMut;

use crate::transport::{Packet, QuickAck, Unpack};
use crate::{mtproto, reader, tl};

#[derive(Debug)]
pub enum PlainDeserializationError {
    QuickAck(QuickAck),
    EncryptedMessage(mtproto::EncryptedMessage),
    Deserialization(tl::de::Error),
}

impl From<tl::de::Error> for PlainDeserializationError {
    fn from(value: tl::de::Error) -> Self {
        Self::Deserialization(value)
    }
}

pub struct DeserializePlain<T: tl::de::Deserialize + Unpin>(PhantomData<T>);

impl<T: tl::de::Deserialize + Unpin> DeserializePlain<T> {
    #[inline]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: tl::de::Deserialize + Unpin> reader::HandleOutput for DeserializePlain<T> {
    type Output = Result<T, PlainDeserializationError>;

    fn acquired(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Output {
        let (data, next) = match unpack {
            Unpack::Packet(Packet { data, next }) => (data, next),
            Unpack::QuickAck(quick_ack) => {
                unsafe { buffer.set_len(0) };

                return Err(PlainDeserializationError::QuickAck(quick_ack));
            }
        };

        let message = match mtproto::Message::unpack(&mut buffer[data.clone()]) {
            mtproto::Message::Plain(message) => message,
            mtproto::Message::Encrypted(message) => {
                unsafe { buffer.set_len(0) };

                return Err(PlainDeserializationError::EncryptedMessage(message));
            }
        };

        let response = tl::de::checked(&buffer[data.start + 20..data.end])?;

        unsafe { buffer.set_len(0) };

        Ok(response)
    }
}
