mod error;

use crate::mtproto::{AuthKey, AuthKeyId, ExternalHeader, InternalHeader, PlainMsgHeader, Side};

use crate::{common, crypto, tl};

use common::infallible;

pub use error::{
    AuthKeyIdError, MessageLengthCheckError, MsgIdCheckError, MsgKeyCheckError, SessionIdError,
};

#[inline]
#[must_use]
pub fn auth_key_id(buf: [u8; 8]) -> Option<AuthKeyId> {
    let auth_key_id = i64::from_le_bytes(buf);

    std::num::NonZeroI64::new(auth_key_id)
}

impl PlainMsgHeader {
    #[inline]
    pub fn unpack(buf: [u8; 12]) -> Self {
        infallible!(Self {
            message_id: i64::from_le_bytes(buf[0..8].try_into().unwrap()),
            message_data_length: i32::from_le_bytes(buf[8..12].try_into().unwrap()),
        })
    }
}

impl ExternalHeader {
    #[inline]
    pub fn unpack(auth_key_id: AuthKeyId, buf: [u8; 16]) -> Self {
        Self {
            auth_key_id,
            msg_key: tl::Int128(buf),
        }
    }

    /// Decrypts the [`ExternalHeader`] using [`AuthKey`] identified by the `auth_key_id` field.
    pub fn decrypt(
        self,
        auth_key: &AuthKey,
        buffer: &mut [u8],
    ) -> Result<InternalHeader, MsgKeyCheckError> {
        assert_eq!(auth_key.id(), self.auth_key_id);

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
