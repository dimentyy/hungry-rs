use std::fmt;

use crate::mtproto;

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

/// Represents an unencrypted message containing only its ID and length.
///
/// ---
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

impl PlainMessage {
    /// Header length of the [`PlainMessage`] in bytes.
    ///
    /// # Header layout
    ///
    /// | auth_key_id | message_id | message_data_length |
    /// |-------------|------------|---------------------|
    /// | int64       | int64      | int32               |
    pub const HEADER_LEN: usize = 8 + 8 + 4;
}

/// Represents an encrypted message containing its `auth_key_id` and `msg_key`.
///
/// ---
/// https://core.telegram.org/mtproto/description#encrypted-message
#[derive(Debug)]
pub struct EncryptedMessage {
    pub auth_key_id: std::num::NonZeroI64,
    pub msg_key: mtproto::MsgKey,
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
    /// Header length of the [`EncryptedMessage`] in bytes.
    ///
    /// # Header layout
    ///
    /// | auth_key_id | msg_key |
    /// |-------------|---------|
    /// | int64       | int128  |
    pub const HEADER_LEN: usize = 8 + 16;
}

/// Represent the data inside an [`EncryptedMessage`] after applying [`decrypt`] method.
///
/// ---
/// https://core.telegram.org/mtproto/description#encrypted-message-encrypted-data
///
/// [`decrypt`]: EncryptedMessage::decrypt
#[derive(Debug)]
pub struct DecryptedMessage {
    pub salt: i64,
    pub session_id: mtproto::Session,
}

impl fmt::Display for DecryptedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "decrypted message [salt={:#016x}, session_id={:016x}]",
            self.salt, self.session_id
        )
    }
}

impl DecryptedMessage {
    /// Header length of the [`DecryptedMessage`] in bytes.
    ///
    /// # Header layout
    ///
    /// | salt  | session_id |
    /// |-------| -----------|
    /// | int64 | int64      |
    pub const HEADER_LEN: usize = 8 + 8;
}
