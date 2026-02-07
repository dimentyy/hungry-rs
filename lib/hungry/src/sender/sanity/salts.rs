use std::cmp::Reverse;
use std::time::SystemTime;

use tracing::{debug, info, warn};

use crate::mtproto::Salt;
use crate::sender::sanity::{BAD_SALT_UNTIL, MAX_GET_FUTURE_SALTS_NUM, Now, Sanity};
use crate::sender::{Handle, SenderError};
use crate::tl;
use crate::transport::Transport;

use tl::SerializedLen;
use tl::mtproto::{funcs, types};

impl<T: Transport, H: Handle> Sanity<T, H> {
    pub(in super::super) fn push_get_future_salts(&mut self) {
        let num = MAX_GET_FUTURE_SALTS_NUM;

        debug!(num, "pushing `get_future_salts#b921bd04`");

        let func = funcs::GetFutureSalts { num };

        let len = func.serialized_len();
        let msg = self.get_msg::<false>(SystemTime::now());

        self.get_container(len)
            .push::<true, _>(&msg, len, |buf| buf.ser(&func));

        self.get_future_salts_msg = Some(msg);
    }

    pub(super) fn bad_server_salt(&mut self, x: types::BadServerSalt) -> Result<(), SenderError> {
        let types::BadServerSalt {
            bad_msg_id: _,
            bad_msg_seqno,
            error_code,
            new_server_salt,
        } = x;

        warn!(
            bad_msg_seqno,
            error_code, "received `bad_server_salt#edab447b`"
        );

        self.future_salts.clear();

        self.reset_salts(new_server_salt);

        self.push_get_future_salts();

        Ok(())
    }

    pub(super) fn handle_future_salts(&mut self, x: types::FutureSalts) -> Result<(), SenderError> {
        let types::FutureSalts {
            req_msg_id,
            now,
            salts,
        } = x;

        info!(len = salts.0.len(), "received `future_salts#ae500895`");

        if let Some(msg) = self.get_future_salts_msg.take() {
            if msg.msg_id != req_msg_id {
                warn!("invalid `req_msg_id`");
            }
        } else {
            warn!("unexpected future salts");
        }

        self.now = Some(Now::new(now));

        self.future_salts = salts.0;
        self.future_salts.sort_by_key(|x| Reverse(x.valid_since));

        self.update_current_salt();

        Ok(())
    }

    fn update_current_salt(&mut self) {
        let Some(ref now) = self.now else {
            return;
        };

        let unix_time = now.unix_time();

        while self.server_salt_until < unix_time {
            let Some(x) = self.future_salts.pop() else {
                break;
            };

            self.server_salt = x.salt;
            self.server_salt_until = x.valid_until;
        }

        if self.future_salts.is_empty() && self.get_future_salts_msg.is_some() {
            self.push_get_future_salts();
        }
    }

    pub(in super::super) fn get_salt(&mut self) -> Salt {
        self.update_current_salt();

        self.server_salt
    }

    #[inline]
    pub(super) const fn reset_salts(&mut self, server_salt: Salt) {
        self.server_salt_until = BAD_SALT_UNTIL;
        self.server_salt = server_salt;
    }
}
