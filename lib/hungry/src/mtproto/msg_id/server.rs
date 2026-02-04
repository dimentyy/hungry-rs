use std::collections::VecDeque;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mtproto::MsgId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MsgIdError {
    Even,
    Negative,
    LowerThanAll,
    EqualToAny,
    InTheFuture,
    InThePast,
}

impl fmt::Display for MsgIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use MsgIdError::*;

        f.write_str("`msg_id` error: ")?;

        f.write_str(match self {
            Even => "even parity",
            Negative => "negative",
            LowerThanAll => "lower than all",
            EqualToAny => "equal to any",
            InTheFuture => "calculated `unix_time` is in the future",
            InThePast => "calculated `unix_time` is in the past",
        })
    }
}

impl std::error::Error for MsgIdError {}

#[must_use]
pub struct ServerMsgIds {
    vec: VecDeque<MsgId>,
    max: MsgId,
    min: MsgId,
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

    #[inline(always)]
    fn gt_max_branch(&mut self, msg_id: MsgId, sys_secs: i32) {
        self.max = msg_id;

        // Not possible to push.
        if self.vec.capacity() == 0 {
            self.min = msg_id;

            return;
        }

        if !self.vec.is_empty() {
            self.drain_front(sys_secs);
        }

        self.vec.push_back(msg_id);
    }

    #[inline(always)]
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

    /// Validates provided [`MsgId`] against provided the `unix_time`.
    ///
    /// # Panics
    ///
    /// * If the [`SystemTime`] is before [`UNIX_EPOCH`].
    ///
    /// # Errors
    ///
    /// See [Security Guidelines] page for information.
    ///
    /// [Security Guidelines]: https://core.telegram.org/mtproto/security_guidelines#checking-msg-id
    pub fn check(&mut self, msg_id: MsgId, unix_time: SystemTime) -> Result<(), MsgIdError> {
        use MsgIdError::*;

        assert!(self.vec.capacity() > 0);

        if msg_id & 1 == 0 {
            return Err(Even);
        }

        if msg_id.is_negative() {
            return Err(Negative);
        }

        let sys_secs: i32 = unix_time
            .duration_since(UNIX_EPOCH)
            .expect("system clock time to be after the Unix epoch")
            .as_secs()
            .try_into()
            .expect("number of secs since the Unix epoch to not overflow");

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

    if sys_secs - 300 > msg_secs {
        return Err(InThePast);
    }

    if msg_secs > sys_secs + 30 {
        return Err(InTheFuture);
    }

    Ok(())
}
