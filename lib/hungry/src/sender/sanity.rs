use std::collections::VecDeque;
use std::mem;

use tracing::{debug, warn};

use crate::sender::{Container, Request, SenderError};
use crate::transport::Transport;
use crate::unpack::MsgContainerIter;
use crate::{mtproto, tl};

use tl::de::Deserialize;
use tl::mtproto::{enums, types};
use tl::{Identifiable, SerializedLen};

pub(super) struct Sanity<T: Transport> {
    pub(super) container: Option<Container<T>>,

    pub(super) client_msg_ids: mtproto::ClientMsgIds,
    pub(super) client_seq_nos: mtproto::SeqNos,

    pub(super) server_msg_ids: mtproto::ServerMsgIds,
    pub(super) server_seq_nos: mtproto::SeqNos,

    pub(super) requests: VecDeque<Request>,

    pub(super) msgs_ack: Vec<mtproto::MsgId>,
}

impl<T: Transport> Sanity<T> {
    #[inline]
    pub(super) fn take_container(&mut self) -> Option<Container<T>> {
        mem::take(&mut self.container)
    }

    pub(super) fn ack(&mut self) {
        if self.msgs_ack.is_empty() {
            return;
        }

        let msg_ids = mem::take(&mut self.msgs_ack);

        debug!(len = msg_ids.len(), "pushing `msgs_ack#62d6b459`");

        let func: enums::MsgsAck = types::MsgsAck { msg_ids }.into();

        let len = func.serialized_len();
        let msg = self.get_msg::<false>();
        let bytes = len.try_into().unwrap();

        let container = if let Some(ref mut container) = self.container {
            container
        } else {
            let container = self.new_container(len);
            self.container.insert(container)
        };

        container.push::<true, _>(&msg, bytes, |buf| buf.ser(&func));

        let enums::MsgsAck::MsgsAck(types::MsgsAck { msg_ids }) = func;

        self.msgs_ack = msg_ids;
        self.msgs_ack.clear();
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    pub(super) fn new_container(&mut self, len: usize) -> Container<T> {
        warn!("TODO: new_container(len={len})");

        // FIXME
        Container::new(unbite::DynBuf::new(len + 4096))
    }

    #[inline]
    pub(super) fn get_msg<const CONTENT_RELATED: bool>(&mut self) -> mtproto::Msg {
        let msg_id = self.client_msg_ids.get(std::time::SystemTime::now());

        let seq_no = if CONTENT_RELATED {
            self.client_seq_nos.get_content_related()
        } else {
            self.client_seq_nos.non_content_related()
        };

        mtproto::Msg { msg_id, seq_no }
    }

    fn handle_single(
        &mut self,
        buf_msg: mtproto::BufMsg<'_>,
        unix_time: std::time::SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
    ) -> Result<(), SenderError> {
        use SenderError::*;

        let mtproto::BufMsg { msg, mut buf, typ } = buf_msg;

        self.check_msg(msg, typ, unix_time)?;

        println!("{typ:#010x}");

        match typ {
            tl::RPC_RESULT => {
                let req_msg_id = buf
                    .de_infallible()
                    .map_err(|err| Deserialization(err.into()))?;

                let typ = buf
                    .de_infallible()
                    .map_err(|err| Deserialization(err.into()))?;

                let object = tl::Object::deserialize(typ, &mut buf).map_err(Deserialization)?;

                self.send_rpc_result(req_msg_id, object);
            }
            types::MsgsAck::CONSTRUCTOR_ID => {}
            types::NewSessionCreated::CONSTRUCTOR_ID => {}
            types::FutureSalts::CONSTRUCTOR_ID => {}
            types::Pong::CONSTRUCTOR_ID => {}

            tl::api::types::Updates::CONSTRUCTOR_ID => {
                updates.push(
                    buf.de::<tl::api::types::Updates>()
                        .map_err(Deserialization)?
                        .into(),
                );
            }

            _ => {
                todo!()
            }
        }

        Ok(())
    }

    fn ungzip<'a>(
        &'_ mut self,
        out: &'a mut Vec<u8>,
        buf_msg: &mut mtproto::BufMsg<'a>,
    ) -> Result<(), SenderError> {
        let bytes = tl::Bytes::deserialize(&mut buf_msg.buf).expect("TODO");
        let buf = bytes.0.as_slice();

        *out = vec![0; dbg!(u32::from_le_bytes(buf[dbg!(buf.len()) - 4..].try_into().unwrap())) as usize];

        let (slice, _) = zlib_rs::decompress_slice(out, buf, Default::default());

        buf_msg.buf = tl::de::Buf::new(slice);
        buf_msg.typ = buf_msg.buf.de_infallible().expect("TODO");

        Ok(())
    }

    fn check_msg(
        &mut self,
        msg: mtproto::Msg,
        typ: u32,
        unix_time: std::time::SystemTime,
    ) -> Result<(), SenderError> {
        self.server_msg_ids.check(msg.msg_id, unix_time)?;
        self.server_seq_nos.check_with_typ(msg, typ)?;

        if msg.seq_no & 1 == 1 {
            self.msgs_ack.push(msg.msg_id);
        }

        Ok(())
    }

    pub(super) fn handle_buf_msg(
        &'_ mut self,
        buf_msg: mtproto::BufMsg<'_>,
        unix_time: std::time::SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
    ) -> Result<(), SenderError> {
        use SenderError::*;

        let mut buf_msg = buf_msg;

        let mut out = Vec::new();

        if buf_msg.typ == tl::GZIP_PACKED {
            self.ungzip(&mut out, &mut buf_msg)?;
        }

        match buf_msg.typ {
            tl::GZIP_PACKED => return Err(DoubleGzipPacked),
            tl::MSG_CONTAINER => {
                let mtproto::BufMsg { msg, buf, typ } = buf_msg;

                let msg_container =
                    MsgContainerIter::new(buf).map_err(|err| Deserialization(err.into()))?;

                // Collect first to validate `bytes` field of `message` type.
                let mut container = Vec::with_capacity(msg_container.len());

                for buf_msg in msg_container {
                    container.push(buf_msg?);
                }

                let mut out = Vec::new();

                for mut buf_msg in container {
                    if buf_msg.typ == tl::GZIP_PACKED {
                        self.ungzip(&mut out, &mut buf_msg)?;
                    }

                    self.handle_single(buf_msg, unix_time, updates)?
                }

                self.check_msg(msg, typ, unix_time)?;
            }
            _ => self.handle_single(buf_msg, unix_time, updates)?,
        }

        Ok(())
    }

    fn send_rpc_result(&mut self, req_msg_id: i64, res_object: tl::Object) {
        let Some(index) = self
            .requests
            .iter()
            .position(|x| x.msg.msg_id == req_msg_id)
        else {
            todo!()
        };

        let request = self.requests.remove(index).unwrap();

        if let Err(_res) = request.tx.send(res_object) {
            todo!()
        }
    }
}
