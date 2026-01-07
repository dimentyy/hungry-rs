use std::fmt;

use crate::mtproto;

/// Represents either [`PlainMessage`] or [`ExternalHeader`] deserialized via [`unpack`] method.
///
/// [`unpack`]: Message::unpack
#[must_use]
#[derive(Debug)]
pub enum Message {
    Plain(PlainMessage),
    Encrypted(ExternalHeader),
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Plain(message) => message.fmt(f),
            Message::Encrypted(message) => message.fmt(f),
        }
    }
}

/// # Unencrypted Messages
///
/// Special plain-text messages may be used to create
/// an authorization key as well as to perform a time
/// synchronization. They begin with auth_key_id = 0 
/// (64 bits) which means that there is no auth_key. 
/// This is followed directly by the message body in 
/// serialized format without internal or external headers.
/// A message identifier (64 bits) and body length in
/// bytes (32 bytes) are added before the message body.
///
/// Only a very limited number of 
/// messages of special types can 
/// be transmitted as plain text.
///
/// ---
/// https://core.telegram.org/mtproto/description#unencrypted-message
#[must_use]
#[derive(Debug)]
pub struct PlainMessage {
    pub id: mtproto::MsgId,
    pub data_length: i32,
}

impl fmt::Display for PlainMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "plain message [id={:#018x}, data_length={}]",
            self.id, self.data_length
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

/// # External (cryptographic) Header
///
/// A header (24 bytes) added before an
/// encrypted message or a container.
/// Consists of the key identifier `auth_key_id` (64 bits)
/// and the message key `msg_key` (128 bits).
///
/// ---
///
/// https://core.telegram.org/mtproto/description#external-cryptographic-header
#[must_use]
#[derive(Debug)]
pub struct ExternalHeader {
    pub auth_key_id: std::num::NonZeroI64,
    pub msg_key: mtproto::MsgKey,
}

impl fmt::Display for ExternalHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "encrypted message [auth_key_id=0x{:016x}, msg_key={:032x}]",
            self.auth_key_id,
            i128::from_ne_bytes(self.msg_key.0)
        )
    }
}

impl ExternalHeader {
    /// Length of the [`ExternalHeader`] in bytes.
    ///
    /// ## Layout
    ///
    /// | auth_key_id | msg_key |
    /// |-------------|---------|
    /// | int64       | int128  |
    ///
    /// ---
    ///
    /// https://core.telegram.org/mtproto/description#encrypted-message
    pub const LEN: usize = 8 + 16;
}

/// # Internal (cryptographic) Header
///
/// A header (16 bytes) added before
/// a message or a container before
/// it is all encrypted together.
/// Consists of the server salt (64 bits)
/// and the session (64 bits).
///
/// ---
///
/// https://core.telegram.org/mtproto/description#internal-cryptographic-header
#[must_use]
#[derive(Debug)]
pub struct InternalHeader {
    pub salt: mtproto::Salt,
    pub session_id: mtproto::Session,
}

impl fmt::Display for InternalHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "decrypted message [salt={:#018x}, session_id={:#018x}]",
            self.salt, self.session_id
        )
    }
}

impl InternalHeader {
    /// Length of the [`InternalHeader`] in bytes.
    ///
    /// ## Layout
    ///
    /// | salt  | session_id |
    /// |-------| -----------|
    /// | int64 | int64      |
    ///
    /// ---
    ///
    /// https://core.telegram.org/mtproto/description#encrypted-message-encrypted-data
    pub const LEN: usize = 8 + 8;
}
