use std::fmt;

use crate::crypto;
use crate::mtproto::{
    AuthKey, DecryptedMessage, EncryptedMessage, Message, MsgKey, PlainMessage, Side,
};

/// # Checking SHA256 hash value of msg_key
///
/// `msg_key` is used not only to compute the AES key and IV
/// to decrypt the received message. After decryption,
/// the client **MUST** check that `msg_key` is indeed
/// equal to SHA256 of the plaintext obtained as the result
/// of decryption (including the final 12...1024 padding bytes),
/// prepended with 32 bytes taken from the `auth_key`,
/// as explained in [MTProto 2.0 Description].
///
/// If an error is encountered before this check could  be performed, the
/// client must perform the `msg_key` check anyway before returning any result.
/// Note that the response to any error encountered before the `msg_key` check
/// must be the same as the response to a failed `msg_key` check.
///
/// ---
/// https://core.telegram.org/mtproto/security_guidelines#checking-sha256-hash-value-of-msg-key
///
/// [MTProto 2.0 Description]: https://core.telegram.org/mtproto/description#defining-aes-key-and-initialization-vector
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MsgKeyCheckError {
    pub computed: MsgKey,
}

impl fmt::Display for MsgKeyCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`msg_key` check error")
    }
}

impl std::error::Error for MsgKeyCheckError {}

impl Message {
    /// Unpacks a [`Message`] enum for working with [`PlainMessage`] and [`EncryptedMessage`].
    pub fn unpack(buffer: &[u8]) -> Message {
        let auth_key_id = i64::from_le_bytes(buffer[0..8].try_into().unwrap());

        let Some(auth_key_id) = std::num::NonZeroI64::new(auth_key_id) else {
            let message_id = i64::from_le_bytes(buffer[8..16].try_into().unwrap());
            let message_length = i32::from_le_bytes(buffer[16..20].try_into().unwrap());

            return Message::Plain(PlainMessage {
                message_id,
                message_length,
            });
        };

        let msg_key = buffer[8..24].try_into().unwrap();

        Message::Encrypted(EncryptedMessage {
            auth_key_id,
            msg_key,
        })
    }
}

impl EncryptedMessage {
    /// Decrypts the [`EncryptedMessage`] using [`AuthKey`] identified by the `auth_key_id` field.
    pub fn decrypt(
        self,
        auth_key: &AuthKey,
        buffer: &mut [u8],
    ) -> Result<DecryptedMessage, MsgKeyCheckError> {
        let (aes_key, mut aes_iv) = auth_key.compute_aes_params(&self.msg_key, Side::Server);

        crypto::aes_ige_decrypt(buffer, &aes_key, &mut aes_iv);

        let computed = auth_key.compute_msg_key(buffer, Side::Server);

        if computed != self.msg_key {
            return Err(MsgKeyCheckError { computed });
        }

        let salt = i64::from_le_bytes(buffer[0..8].try_into().unwrap());
        let session_id = i64::from_le_bytes(buffer[8..16].try_into().unwrap());

        Ok(DecryptedMessage { salt, session_id })
    }
}
