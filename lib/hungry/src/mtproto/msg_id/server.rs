use std::cmp::Ordering;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::mtproto;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MsgIdError {
    Client,
    InvalidMod,
    LowerThanAll,
    EqualToAny,
    InTheFuture,
    InThePast,
}

#[must_use]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MsgIdModulus {
    Response = 1,
    Other = 3,
}

#[must_use]
pub struct ServerMsgIds {
    vec: VecDeque<mtproto::MsgId>,
    max: mtproto::MsgId,
    min: mtproto::MsgId,
}

impl ServerMsgIds {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self {
            vec: VecDeque::with_capacity(capacity),
            max: mtproto::MsgId::MIN,
            min: mtproto::MsgId::MIN,
        }
    }

    #[inline]
    fn push(&mut self, msg_id: mtproto::MsgId) {
        if self.vec.capacity() == 0 {
            return;
        }

        if self.vec.len() == self.vec.capacity() {
            let _ = self.vec.pop_front();
        }

        self.vec.push_front(msg_id);
    }

    /// Returns [`MsgIdModulus`] of the provided [`MsgId`].
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
    pub fn validate(
        &mut self,
        msg_id: mtproto::MsgId,
        unix_time: SystemTime,
    ) -> Result<MsgIdModulus, MsgIdError> {
        use Ordering::*;

        use MsgIdError::*;

        let modulus = match msg_id & 3 {
            0 => return Err(Client),
            1 => MsgIdModulus::Response,
            2 => return Err(InvalidMod),
            3 => MsgIdModulus::Other,
            _ => unreachable!(),
        };

        let sys_secs = unix_time
            .duration_since(UNIX_EPOCH)
            .expect("system clock time to be after the Unix epoch")
            .as_secs();

        let msg_secs = msg_id.cast_unsigned() >> 32;

        if sys_secs - 300 > msg_secs {
            return Err(InThePast);
        }

        if msg_secs > sys_secs + 30 {
            return Err(InTheFuture);
        }

        match msg_id.cmp(&self.max) {
            Less => {}
            Equal => return Err(EqualToAny),
            Greater => {
                self.push(msg_id);

                return Ok(modulus);
            }
        }

        match msg_id.cmp(&self.min) {
            Less => return Err(LowerThanAll),
            Equal => return Err(EqualToAny),
            Greater => {}
        }

        // Probably better to iterate backwards, though it is not a happy path.
        if self.vec.contains(&msg_id) {
            return Err(EqualToAny);
        }

        self.push(msg_id);

        Ok(modulus)
    }
}
