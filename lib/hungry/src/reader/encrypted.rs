use std::fmt;

use tokio::io::AsyncRead;

use crate::mtproto::{
    AuthKey, AuthKeyIdError, ExternalHeader, InternalHeader, MsgKeyCheckError, unpack_auth_key_id,
};
use crate::reader::Reader;
use crate::transport::{Packet, Transport};
use crate::{common, tl};

use common::infallible;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncryptedMessageError {
    TooSmall { len: usize },
    Plaintext,
    AuthKeyId(AuthKeyIdError),
    InvalidLen { len: usize },
    MsgKeyCheck(MsgKeyCheckError),
}

impl fmt::Display for EncryptedMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use EncryptedMessageError::*;

        f.write_str("packet error: ")?;

        match self {
            TooSmall { len } => write!(f, "too small: {len} bytes"),
            Plaintext => f.write_str("plaintext message"),
            AuthKeyId(err) => err.fmt(f),
            InvalidLen { len } => write!(f, "invalid len: {len} bytes"),
            MsgKeyCheck(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for EncryptedMessageError {
    #[expect(clippy::match_same_arms)]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use EncryptedMessageError::*;

        match self {
            TooSmall { .. } => None,
            Plaintext => None,
            AuthKeyId(err) => Some(err),
            InvalidLen { .. } => None,
            MsgKeyCheck(err) => Some(err),
        }
    }
}

impl<R: AsyncRead + Unpin, T: Transport> Reader<R, T> {
    /// # Panics
    ///
    /// * If the provided [`Packet`] is invalid or [`Reader`] has advanced.
    #[track_caller]
    pub fn encrypted_message(
        &mut self,
        packet: Packet,
        auth_key: &AuthKey,
    ) -> Result<(InternalHeader, tl::de::Buf<'_>), EncryptedMessageError> {
        use EncryptedMessageError::*;

        let buf = &mut self.buffer.as_mut_slice()[packet.data];

        if buf.len() < ExternalHeader::LEN + InternalHeader::LEN {
            return Err(TooSmall { len: buf.len() });
        }

        infallible! {
            let (auth_key_id, buf) = buf.split_first_chunk_mut().unwrap();
        }

        let Some(auth_key_id) = unpack_auth_key_id(*auth_key_id) else {
            return Err(Plaintext);
        };

        if auth_key_id != auth_key.id() {
            return Err(AuthKeyId(AuthKeyIdError(auth_key_id)));
        }

        infallible! {
            let (external, buf) = buf.split_first_chunk_mut().unwrap();
        }

        let external = ExternalHeader::unpack(auth_key_id, *external);

        let len = buf.len();

        if !len.is_multiple_of(16) {
            return Err(InvalidLen { len });
        }

        external.decrypt(auth_key, buf).map_err(MsgKeyCheck)?;

        infallible! {
            let (internal, buf) = buf.split_first_chunk_mut().unwrap();
        }

        let internal = InternalHeader::unpack(*internal);

        let buf = tl::de::Buf::new(buf);

        Ok((internal, buf))
    }
}
