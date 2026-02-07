use std::mem;
use std::time::SystemTime;

use tracing::debug;

use crate::mtproto::{MAX_IDS_PER_SERVICE_MSG, Msg, is_content_related};
use crate::sender::{Handle, Sanity, SenderError};
use crate::tl;
use crate::transport::Transport;

use tl::SerializedLen;
use tl::mtproto::{enums, types};

impl<T: Transport, H: Handle> Sanity<T, H> {
    pub(super) fn msgs_ack(&mut self, x: types::MsgsAck) -> Result<(), SenderError> {
        let types::MsgsAck { msg_ids } = x;

        debug!(len = msg_ids.len(), "received `msgs_ack#62d6b459`");

        Ok(())
    }

    pub(in super::super) fn push_msgs_ack(&mut self) {
        if self.msgs_ack_msg_ids.is_empty() {
            return;
        }

        let msg_ids = mem::take(&mut self.msgs_ack_msg_ids);

        debug!(len = msg_ids.len(), "pushing `msgs_ack#62d6b459`");

        let func: enums::MsgsAck = types::MsgsAck { msg_ids }.into();

        let len = func.serialized_len();
        let msg = self.get_msg::<false>(SystemTime::now());

        self.get_container(len)
            .push::<true, _>(&msg, len, |buf| buf.ser(&func));

        let enums::MsgsAck::MsgsAck(types::MsgsAck { msg_ids }) = func;

        self.msgs_ack_msg_ids = msg_ids;
        self.msgs_ack_msg_ids.clear();
    }

    pub(super) fn ack(&mut self, msg: Msg) {
        if !is_content_related(msg.seq_no) {
            return;
        }

        let len = self.msgs_ack_msg_ids.len();

        self.msgs_ack_msg_ids.push(msg.msg_id);

        if len < MAX_IDS_PER_SERVICE_MSG {
            return;
        }

        self.push_msgs_ack();

        // TODO: handle `BadMsgNotification` to resend the message.
    }
}
