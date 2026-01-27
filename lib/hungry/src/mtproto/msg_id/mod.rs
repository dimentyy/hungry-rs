mod client;
mod server;

use std::time::{SystemTime, UNIX_EPOCH};

pub use client::ClientMsgIds;
pub use server::{MsgIdError, ServerMsgIds};

/// # Message Identifier (msg_id)
///
/// A (time-dependent) 64-bit number used uniquely to identify a
/// message within a session. Client message identifiers are divisible
/// by 4, server message identifiers modulo 4 yield 1 if the message
/// is a response to a client message, and 3 otherwise. Client message
/// identifiers must increase monotonically (within a single session),
/// the same as server message identifiers, and must approximately equal
/// unixtime*2^32. This way, a message identifier points to the approximate
/// moment in time the message was created. A message is rejected over
/// 300 seconds after it is created or 30 seconds before it is created
/// (this is needed to protect from replay attacks). In this situation,
/// it must be re-sent with a different identifier (or placed in a
/// container with a higher identifier). The identifier of a message
/// container must be strictly greater than those of its nested messages.
///
/// **Important:** to counter replay-attacks the lower 32 bits of
/// **msg_id** passed by the client must not be empty and must present
/// a fractional part of the time point when the message was created.
///
/// ---
///
/// <https://core.telegram.org/mtproto/description#message-identifier-msg-id>
pub type MsgId = i64;

/// Calculates a new [`MsgId`] from given `unix_time`.
///
/// # Panics
///
/// * If the [`SystemTime`] is before [`UNIX_EPOCH`].
#[inline]
#[must_use]
pub fn msg_id(unix_time: SystemTime) -> MsgId {
    let unix_time = unix_time
        .duration_since(UNIX_EPOCH)
        .expect("system clock time to be after the Unix epoch");

    let secs = unix_time.as_secs().cast_signed();
    let subsec_nanos = i64::from(unix_time.subsec_nanos());

    secs << 32 | subsec_nanos << 2
}
