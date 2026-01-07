mod error;

use crate::mtproto::{
    AuthKey, InternalHeader, ExternalHeader, Message, MsgKey, PlainMessage, Side,
};

use crate::{crypto, tl};

pub use error::{MessageLengthCheckError, MsgIdCheckError, MsgKeyCheckError};

impl Message {
    /// Unpacks a [`Message`] enum for working with [`PlainMessage`] and [`ExternalHeader`].
    pub fn unpack(buffer: &[u8]) -> Message {
        let auth_key_id = i64::from_le_bytes(buffer[0..8].try_into().unwrap());

        let Some(auth_key_id) = std::num::NonZeroI64::new(auth_key_id) else {
            let id = i64::from_le_bytes(buffer[8..16].try_into().unwrap());
            let data_length = i32::from_le_bytes(buffer[16..20].try_into().unwrap());

            return Message::Plain(PlainMessage { id, data_length });
        };

        let msg_key = tl::Int128(buffer[8..24].try_into().unwrap());

        Message::Encrypted(ExternalHeader {
            auth_key_id,
            msg_key,
        })
    }
}

impl ExternalHeader {
    /// Decrypts the [`ExternalHeader`] using [`AuthKey`] identified by the `auth_key_id` field.
    pub fn decrypt(
        self,
        auth_key: &AuthKey,
        buffer: &mut [u8],
    ) -> Result<(InternalHeader), MsgKeyCheckError> {
        let (aes_key, mut aes_iv) = auth_key.compute_aes_params(&self.msg_key, Side::Server);

        crypto::aes_ige_decrypt(buffer, &aes_key, &mut aes_iv);

        let computed = auth_key.compute_msg_key(buffer, Side::Server);

        if computed != self.msg_key {
            return Err(MsgKeyCheckError { computed });
        }

        let salt = i64::from_le_bytes(buffer[0..8].try_into().unwrap());
        let session_id = i64::from_le_bytes(buffer[8..16].try_into().unwrap());

        Ok(InternalHeader { salt, session_id })
    }
}
