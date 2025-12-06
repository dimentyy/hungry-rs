use std::fmt;
use std::num::NonZeroI64;

use crate::utils::SliceExt;
use crate::{crypto, mtproto, tl};

/// Represents either [`PlainMessage`] or [`EncryptedMessage`] deserialized via [`unpack`] method.
///
/// [`unpack`]: Message::unpack
#[derive(Debug)]
pub enum Message {
    Plain(PlainMessage),
    Encrypted(EncryptedMessage),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Plain(message) => message.fmt(f),
            Message::Encrypted(message) => message.fmt(f),
        }
    }
}

impl Message {
    /// Unpacks a [`Message`] enum for working with [`PlainMessage`] and [`EncryptedMessage`].
    pub fn unpack(buffer: &[u8]) -> Message {
        let Some(auth_key_id) = NonZeroI64::new(i64::from_le_bytes(*buffer[0..8].arr())) else {
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

/// Represents an unencrypted message containing only its ID and length. \
/// https://core.telegram.org/mtproto/description#unencrypted-message
#[derive(Debug)]
pub struct PlainMessage {
    pub message_id: i64,
    pub message_length: i32,
}

impl fmt::Display for PlainMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "plain message [message_id=0x{:016x}, message_length={}]",
            self.message_id, self.message_length
        )
    }
}

/// Represents an encrypted message containing its `auth_key_id` and `msg_key`. \
/// https://core.telegram.org/mtproto/description#encrypted-message
#[derive(Debug)]
pub struct EncryptedMessage {
    pub auth_key_id: NonZeroI64,
    pub msg_key: tl::Int128,
}

impl fmt::Display for EncryptedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "encrypted message [auth_key_id=0x{:016x}, msg_key={:032x}]",
            self.auth_key_id,
            i128::from_ne_bytes(self.msg_key)
        )
    }
}

impl EncryptedMessage {
    /// Decrypts the [`EncryptedMessage`] using [`AuthKey`] identified by the `auth_key_id` field.
    ///
    /// [`AuthKey`]: mtproto::AuthKey
    pub fn decrypt(self, auth_key: &mtproto::AuthKey, buffer: &mut [u8]) -> DecryptedMessage {
        assert!(buffer.len() >= 40);

        let (aes_key, aes_iv) = auth_key.compute(&self.msg_key, mtproto::Side::Server);

        crypto::aes_ige_decrypt(buffer, &aes_key, &aes_iv);

        let salt = i64::from_le_bytes(*buffer[0..8].arr());
        let session_id = i64::from_le_bytes(*buffer[8..16].arr());
        let message_id = i64::from_le_bytes(*buffer[16..24].arr());
        let seq_no = i64::from_le_bytes(*buffer[24..32].arr());
        let message_length = i64::from_le_bytes(*buffer[32..40].arr());

        DecryptedMessage {
            salt,
            session_id,
            message_id,
            seq_no,
            message_length,
        }
    }
}

/// Represent the data inside an [`EncryptedMessage`] after applying [`decrypt`] method. \
/// https://core.telegram.org/mtproto/description#encrypted-message-encrypted-data
///
/// [`decrypt`]: EncryptedMessage::decrypt
#[derive(Debug)]
pub struct DecryptedMessage {
    pub salt: i64,
    pub session_id: i64,
    pub message_id: i64,
    pub seq_no: i64,
    pub message_length: i64,
}

impl fmt::Display for DecryptedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "decrypted message [salt={:#016x}, session_id={:016x}, message_id={:#016x}, seq_no={}, message_length={}]",
            self.salt, self.session_id, self.message_id, self.seq_no, self.message_length
        )
    }
}
