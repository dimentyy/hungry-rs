use crate::crypto;
use crate::mtproto::{AuthKey, DecryptedMessage, EncryptedMessage, Message, PlainMessage, Side};
use crate::utils::SliceExt;

impl Message {
    /// Unpacks a [`Message`] enum for working with [`PlainMessage`] and [`EncryptedMessage`].
    #[must_use]
    pub fn unpack(buffer: &[u8]) -> Message {
        let auth_key_id = i64::from_le_bytes(*buffer[0..8].arr());

        let Some(auth_key_id) = std::num::NonZeroI64::new(auth_key_id) else {
            let message_id = i64::from_le_bytes(*buffer[8..16].arr());
            let message_length = i32::from_le_bytes(*buffer[16..20].arr());

            return Message::Plain(PlainMessage {
                message_id,
                message_length,
            });
        };

        let msg_key = *buffer[8..24].arr();

        Message::Encrypted(EncryptedMessage {
            auth_key_id,
            msg_key,
        })
    }
}

impl EncryptedMessage {
    /// Decrypts the [`EncryptedMessage`] using [`AuthKey`] identified by the `auth_key_id` field.
    #[must_use]
    pub fn decrypt(self, auth_key: &AuthKey, buffer: &mut [u8]) -> DecryptedMessage {
        assert!(buffer.len() >= 40);

        let (aes_key, aes_iv) = auth_key.compute_aes_params(&self.msg_key, Side::Server);

        crypto::aes_ige_decrypt(buffer, &aes_key, &aes_iv);

        let salt = i64::from_le_bytes(*buffer[0..8].arr());
        let session_id = i64::from_le_bytes(*buffer[8..16].arr());

        DecryptedMessage { salt, session_id }
    }
}
