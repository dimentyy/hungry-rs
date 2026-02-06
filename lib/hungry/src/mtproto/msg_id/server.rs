use std::collections::VecDeque;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mtproto::{MsgId, MsgIdError, REJECT_MSG_ID_AFTER, REJECT_MSG_ID_UNTIL};

#[must_use]
pub struct ServerMsgIds {
    vec: VecDeque<MsgId>,
    max: MsgId,
    min: MsgId,
}

impl fmt::Debug for ServerMsgIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServerMsgIds")
            .field("max", &format_args!("{:#018x}", self.max))
            .field("min", &format_args!("{:#018x}", self.min))
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ServerMsgIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "server msg ids [max={:#018x}, min={:#018x}, ..]",
            self.max, self.min
        )
    }
}

impl ServerMsgIds {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self {
            vec: VecDeque::with_capacity(capacity),
            max: i64::MIN,
            min: i64::MIN,
        }
    }

    #[inline]
    #[must_use]
    pub const fn max(&self) -> MsgId {
        self.max
    }

    #[inline]
    #[must_use]
    pub const fn min(&self) -> MsgId {
        self.min
    }

    #[inline]
    #[must_use]
    pub const fn vec(&self) -> &VecDeque<MsgId> {
        &self.vec
    }

    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear();
        self.max = i64::MIN;
        self.min = i64::MIN;
    }

    #[inline(always)]
    fn gt_max_branch(&mut self, msg_id: MsgId, sys_secs: i32) {
        self.max = msg_id;

        // Impossible to push.
        if self.vec.capacity() == 0 {
            self.min = msg_id;

            return;
        }

        if !self.vec.is_empty() {
            self.drain_front(sys_secs);
        }

        self.vec.push_back(msg_id);
    }

    fn drain_front(&mut self, sys_secs: i32) {
        // Do not reserve extra, pop the front immediately.
        if self.vec.len() == self.vec.capacity() {
            let _ = self.vec.pop_front();
        }

        let index = self.vec.partition_point(|&x| is_in_the_past(x, sys_secs));

        if let Some(last) = self.vec.drain(..index).next_back() {
            self.min = last;
        }
    }

    /// # Panics
    ///
    /// * If the [`SystemTime`] exceeds signed 32-bit Unix timestamp range.
    ///
    /// # Errors
    ///
    /// See the [Security Guidelines] page for information.
    ///
    /// [Security Guidelines]: https://core.telegram.org/mtproto/security_guidelines#checking-msg-id
    pub fn check(&mut self, msg_id: MsgId, system_time: SystemTime) -> Result<(), MsgIdError> {
        use MsgIdError::*;

        assert!(self.vec.capacity() > 0);

        if msg_id & 1 == 0 {
            return Err(Even);
        }

        if msg_id.is_negative() {
            return Err(Negative);
        }

        let sys_secs: i32 = system_time
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap();

        check_unix_time(msg_id, sys_secs)?;

        if self.vec.is_empty() {
            self.vec.push_back(msg_id);

            return Ok(());
        }

        // Should be the hot path.
        if msg_id > self.max {
            self.gt_max_branch(msg_id, sys_secs);

            return Ok(());
        }

        // Unless `self.min` is set, this will not error.
        if msg_id < self.min {
            return Err(LowerThanAll);
        }

        // Range bounds `self.max` & `self.min` are equal.
        if self.vec.capacity() == 0 {
            return Err(EqualToAny);
        }

        if !self.vec.is_empty() {
            self.drain_front(sys_secs);
        }

        let index = match self.vec.binary_search(&msg_id) {
            Ok(_pos) => return Err(EqualToAny),
            Err(pos) => pos,
        };

        self.vec.insert(index, msg_id);

        Ok(())
    }
}

#[inline(always)]
const fn is_in_the_past(msg_id: MsgId, sys_secs: i32) -> bool {
    let msg_secs = (msg_id >> 32) as i32;

    sys_secs - 300 > msg_secs
}

#[inline(always)]
const fn check_unix_time(msg_id: MsgId, sys_secs: i32) -> Result<(), MsgIdError> {
    use MsgIdError::*;

    let msg_secs = (msg_id >> 32) as i32;

    if msg_secs < sys_secs - REJECT_MSG_ID_AFTER {
        return Err(InThePast);
    }

    if sys_secs + REJECT_MSG_ID_UNTIL < msg_secs {
        return Err(InTheFuture);
    }

    Ok(())
}
