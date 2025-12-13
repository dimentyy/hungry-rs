// STATUS: stable.

use std::{fmt, time};

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
/// https://core.telegram.org/mtproto/description#message-identifier-msg-id
#[must_use]
#[derive(Default)]
pub struct MsgIds {
    last: i64,
}

impl fmt::Debug for MsgIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MsgIds")
            .field("previous", &format_args!("{:#016x}", self.last))
            .finish()
    }
}

impl MsgIds {
    #[inline]
    pub const fn new() -> Self {
        Self { last: 0 }
    }

    #[inline]
    #[must_use]
    pub const fn last(&self) -> i64 {
        self.last
    }

    #[must_use]
    pub const fn get(&mut self, unix_time: time::Duration) -> i64 {
        let secs = unix_time.as_secs() as i64;
        let subsec_nanos = unix_time.subsec_nanos() as i64;

        let message_id = secs << 32 | subsec_nanos << 2;

        if self.last >= message_id {
            self.last += 4;

            self.last
        } else {
            self.last = message_id;

            message_id
        }
    }

    #[must_use]
    pub fn get_using_system_time(&mut self) -> i64 {
        let unix_time = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .expect("system clock time to be after the Unix epoch");

        self.get(unix_time)
    }
}
