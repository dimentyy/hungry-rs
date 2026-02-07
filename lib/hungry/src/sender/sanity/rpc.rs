use tracing::error;

use crate::mtproto::Msg;
use crate::sender::{Handle, Request, Sanity, SenderError};
use crate::tl;
use crate::transport::Transport;

impl<T: Transport, H: Handle> Sanity<T, H> {
    pub(in super::super) fn rpc_request(&mut self, msg: Msg, extra: H::RpcExtra) {
        let request = Request { msg, extra };

        self.requests.push_back(request);
    }

    pub(super) fn rpc_result(
        &mut self,
        buf: tl::de::Buf<'_>,
        handle: &mut H,
    ) -> Result<(), SenderError> {
        let mut buf = buf;

        let req_msg_id = buf.de()?;

        let mut typ = buf.de()?;

        let Some(index) = self
            .requests
            .iter()
            .position(|x| x.msg.msg_id == req_msg_id)
        else {
            error!(
                req_msg_id,
                "received `rpc_result#f35c6d01` with unknown `req_msg_id`"
            );

            return Ok(());
        };

        let req = self.requests.swap_remove_front(index).unwrap();

        let mut out = None;

        if typ == tl::GZIP_PACKED {
            typ = self.ungzip_packed_bytes(&mut out, &mut buf)?;
        }

        handle.rpc_result(req_msg_id, req.extra, typ, &mut buf);

        if let Some(out) = out {
            self.return_temporary_buffer(out);
        }

        Ok(())
    }
}
