use tracing::error;

use crate::mtproto::Msg;
use crate::sender::{Handle, Sanity, SenderError};
use crate::tl;
use crate::transport::Transport;

use tl::Identifiable;
use tl::mtproto::types;

pub(super) struct Request<H: Handle> {
    pub(super) msg: Msg,
    pub(super) container_msg: Option<Msg>,
    pub(super) extra: H::Extra,
}

impl<T: Transport, H: Handle> Sanity<T, H> {
    pub(in super::super) fn rpc_request(&mut self, msg: Msg, extra: H::Extra) {
        let request = Request {
            msg,
            container_msg: None,
            extra,
        };

        self.requests.push_back(request);
    }

    pub(super) fn rpc_result(
        &mut self,
        buf: tl::de::Buf<'_>,
        handle: &mut H,
    ) -> Result<(), SenderError> {
        use SenderError::*;

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

        match typ {
            tl::GZIP_PACKED => return Err(DoubleGzipPacked),
            tl::MSG_CONTAINER => todo!(),

            types::RpcError::CONSTRUCTOR_ID => {
                let error = buf.de()?;

                handle.rpc_result_error(req.extra, error);
            }
            _ => {
                handle.rpc_result(req.extra, typ, &mut buf);
            }
        }

        if let Some(out) = out {
            self.push_buffer(out.into_raw());
        }

        Ok(())
    }
}
