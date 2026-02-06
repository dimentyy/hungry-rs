use std::fmt;
use std::time::SystemTime;

use crate::mtproto::{MsgId, new_msg_id};

#[must_use]
pub struct ClientMsgIds {
    last: MsgId,
}

impl fmt::Debug for ClientMsgIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientMsgIds")
            .field("last", &format_args!("{:#018x}", self.last))
            .finish()
    }
}

impl fmt::Display for ClientMsgIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client msg ids [last={:#018x}]", self.last)
    }
}

impl ClientMsgIds {
    /// # Panics
    ///
    /// If the [`SystemTime`] exceeds signed 32-bit Unix timestamp range.
    #[inline]
    pub fn new(unix_time: SystemTime) -> Self {
        Self {
            last: new_msg_id(unix_time),
        }
    }

    #[inline]
    #[must_use]
    pub const fn last(&self) -> MsgId {
        self.last
    }

    /// # Panics
    ///
    /// If the [`SystemTime`] exceeds signed 32-bit Unix timestamp range.
    #[must_use]
    pub fn get(&mut self, system_time: SystemTime) -> MsgId {
        let msg_id = new_msg_id(system_time);

        if msg_id <= self.last {
            self.last += 4;

            self.last
        } else {
            self.last = msg_id;

            msg_id
        }
    }
}
