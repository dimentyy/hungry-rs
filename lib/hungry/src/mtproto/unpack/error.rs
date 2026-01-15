use std::fmt;
use std::num::NonZeroI64;

use crate::mtproto;

/// # Checking session_id
///
/// The client is to check that the `session_id`
/// field in the decrypted message indeed equals to
/// that of an active session created by the client.
///
/// ---
///
/// <https://core.telegram.org/mtproto/security_guidelines#checking-session-id>
#[derive(Debug, Eq, PartialEq)]
pub struct SessionIdError(pub mtproto::Session);

impl fmt::Display for SessionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "received unexpected `session_id`: {:#018x}", self.0)
    }
}

impl std::error::Error for SessionIdError {}

#[derive(Debug, Eq, PartialEq)]
pub struct AuthKeyIdError(pub Option<mtproto::AuthKeyId>);

impl fmt::Display for AuthKeyIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let auth_key_id = self.0.map_or(0, NonZeroI64::get);

        write!(f, "received unexpected `auth_key_id`: {auth_key_id:#018x}")
    }
}

impl std::error::Error for AuthKeyIdError {}

/// # Checking SHA256 hash value of msg_key
///
/// `msg_key` is used not only to compute the AES key and IV to
/// decrypt the received message. After decryption, the client
/// **MUST** check that `msg_key` is indeed equal to SHA256 of the
/// plaintext obtained as the result of decryption (including the
/// final 12...1024 padding bytes), prepended with 32 bytes taken
/// from the `auth_key`, as explained in [MTProto 2.0 Description].
///
/// If an error is encountered before this check could  be performed, the
/// client must perform the `msg_key` check anyway before returning any result.
/// Note that the response to any error encountered before the `msg_key`
/// check must be the same as the response to a failed `msg_key` check.
///
/// ---
///
/// <https://core.telegram.org/mtproto/security_guidelines#checking-sha256-hash-value-of-msg-key>
///
/// [MTProto 2.0 Description]: https://core.telegram.org/mtproto/description#defining-aes-key-and-initialization-vector
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MsgKeyCheckError {
    pub computed: mtproto::MsgKey,
}

impl fmt::Display for MsgKeyCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`msg_key` check error")
    }
}

impl std::error::Error for MsgKeyCheckError {}

/// # Checking message length
///
/// The client **must** check that the length of the
/// message or container obtained from the decrypted
/// message (computed from its `length` field) does not
/// exceed the total size of the plaintext, and that the
/// difference (i.e. the length of the random padding)
/// lies in the range from 12 to 1024 bytes.
///
/// The length should be always divisible by 4 and non-negative.
/// On no account the client is to access data past the end
/// of the decryption buffer containing the plaintext message.
///
/// ---
///
/// <https://core.telegram.org/mtproto/security_guidelines#checking-message-length>
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageLengthCheckError {
    pub received: i32,
}

impl fmt::Display for MessageLengthCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("message length check error")
    }
}

impl std::error::Error for MessageLengthCheckError {}

/// # Checking msg_id
///
/// The client must check that `msg_id` has even parity for messages from
/// client to server, and odd parity for messages from server to client.
///
/// In addition, the identifiers (msg_id) of the last N messages
/// received from the other side must be stored, and if a message comes
/// in with a msg_id lower than all or equal to any of the  stored
/// values, that message is to be ignored. Otherwise, the new message
/// msg_id is added to the set, and, if the number of stored msg_id
/// values is greater than N, the oldest (i.e. the lowest) is discarded.
///
/// In addition, msg_id values that belong over 30 seconds in the future
/// or over 300 seconds in the past are to be ignored (recall that `msg_id`
/// approximately equals unixtime * 2^32). This is especially important
/// for the server. The client would also find this useful (to protect
/// from a replay attack), but only if it is certain of its time (for
/// example, if its time has been synchronized with that of the server).
///
/// Certain client-to-server service messages containing data sent
/// by the client to the server (for example, `msg_id` of a recent
/// client query) may, nonetheless, be processed on the client even
/// if the time appears to be “incorrect”. This is especially true
/// of messages to change server_salt and notifications about invalid
/// time on the client. See [Mobile Protocol: Service Messages].
///
/// ---
///
/// <https://core.telegram.org/mtproto/security_guidelines#checking-msg-id>
///
/// [Mobile Protocol: Service Messages]: https://core.telegram.org/mtproto/service_messages
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MsgIdCheckError {
    pub received: mtproto::MsgId,
}

impl fmt::Display for MsgIdCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`msg_id` check error")
    }
}

impl std::error::Error for MsgIdCheckError {}
