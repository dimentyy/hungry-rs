use std::fmt;

use crate::mtproto;

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
///
/// <https://core.telegram.org/mtproto/description#unencrypted-messages>
#[must_use]
#[derive(Debug)]
pub struct UnencryptedMessage {
    pub id: mtproto::MsgId,
    pub data_length: i32,
}

impl fmt::Display for UnencryptedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unencrypted message [id={:#018x}, data_length={}]",
            self.id, self.data_length
        )
    }
}

impl UnencryptedMessage {
    /// Length of the [`UnencryptedMessage`] in bytes.
    ///
    /// # Layout
    ///
    /// | auth_key_id = `0` | message_id | message_data_length |
    /// |-------------------|------------|---------------------|
    /// | int64             | int64      | int32               |
    ///
    /// ---
    ///
    /// <https://core.telegram.org/mtproto/description#unencrypted-message>
    pub const LEN: usize = 8 + 8 + 4;
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
/// <https://core.telegram.org/mtproto/description#external-cryptographic-header>
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
            "external header [auth_key_id=0x{:016x}, msg_key={:032x}]",
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
    /// <https://core.telegram.org/mtproto/description#encrypted-message>
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
/// <https://core.telegram.org/mtproto/description#internal-cryptographic-header>
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
            "internal header [salt={:#018x}, session_id={:#018x}]",
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
    /// <https://core.telegram.org/mtproto/description#encrypted-message-encrypted-data>
    pub const LEN: usize = 8 + 8;
}
