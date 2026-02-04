use std::fmt;

use tokio::io::AsyncRead;

use crate::mtproto::{AuthKeyIdError, UnencryptedMessage, unpack_auth_key_id};
use crate::reader::Reader;
use crate::transport::{Packet, Transport};
use crate::{common, mtproto, tl};

use common::infallible;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlaintextMessageError {
    TooSmall { len: usize },
    AuthKeyId(AuthKeyIdError),
    InvalidDataLength,
}

impl fmt::Display for PlaintextMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PlaintextMessageError::*;

        f.write_str("packet error: ")?;

        match self {
            TooSmall { len } => write!(f, "too small: {len} bytes"),
            AuthKeyId(err) => err.fmt(f),
            InvalidDataLength => f.write_str("invalid `data_length`"),
        }
    }
}

impl std::error::Error for PlaintextMessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use PlaintextMessageError::*;

        match self {
            TooSmall { .. } => None,
            AuthKeyId(err) => Some(err),
            InvalidDataLength => None,
        }
    }
}

impl<R: AsyncRead + Unpin, T: Transport> Reader<R, T> {
    pub fn plaintext_message(
        &'_ self,
        packet: Packet,
    ) -> Result<(mtproto::MsgId, tl::de::Buf<'_>), PlaintextMessageError> {
        use PlaintextMessageError::*;

        let buf = &self.buffer.as_slice()[packet.data];

        if buf.len() < UnencryptedMessage::LEN {
            return Err(TooSmall { len: buf.len() });
        }

        infallible! {
            let (auth_key_id, buf) = buf.split_first_chunk().unwrap();
        }

        if let Some(auth_key_id) = unpack_auth_key_id(*auth_key_id) {
            return Err(AuthKeyId(AuthKeyIdError(auth_key_id)));
        }

        infallible! {
            let (header, buf) = buf.split_first_chunk().unwrap();
        }

        let message = UnencryptedMessage::unpack(*header);

        let Ok(data_length) = usize::try_from(message.data_length) else {
            return Err(InvalidDataLength);
        };

        if data_length != buf.len() {
            return Err(InvalidDataLength);
        }

        if !data_length.is_multiple_of(4) {
            return Err(InvalidDataLength);
        }

        let buf = tl::de::Buf::new(buf);

        Ok((message.id, buf))
    }
}
