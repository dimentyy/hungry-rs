mod ack;
mod now;
mod rpc;
mod salts;

use std::collections::VecDeque;
use std::mem;
use std::time::SystemTime;

use crate::mtproto::{
    BufMsg, ClientMsgIds, ClientSeqNos, MAX_IDS_PER_SERVICE_MSG, Msg, MsgId, Salt, SeqNoError,
    ServerMsgIds, ServerSeqNos,
};
use crate::sender::{Container, SenderError};
use crate::tl;
use crate::transport::Transport;
use crate::unpack::{MsgContainerIter, ungzip};

use tracing::{info, warn};

use tl::Identifiable;
use tl::de::Deserialize;
use tl::mtproto::types;

use now::Now;
use rpc::Request;

const BAD_SALT_UNTIL: i32 = i32::MIN;
const MAX_GET_FUTURE_SALTS_NUM: i32 = 64;

pub trait Handle {
    type RpcResultExtra;

    fn rpc_result(
        &mut self,
        msg_id: MsgId,
        extra: Self::RpcResultExtra,
        typ: u32,
        buf: &mut tl::de::Buf<'_>,
    );

    fn rpc_result_error(
        &mut self,
        msg_id: MsgId,
        extra: Self::RpcResultExtra,
        error: types::RpcError,
    );
}

// FIXME: this struct manages too many things right now.
pub(super) struct Sanity<T: Transport, H: Handle> {
    pub(super) container: Option<Container<T>>,

    client_msg_ids: ClientMsgIds,
    client_seq_nos: ClientSeqNos,

    server_msg_ids: ServerMsgIds,
    server_seq_nos: ServerSeqNos,

    now: Option<Now>,

    get_future_salts_msg: Option<Msg>,

    future_salts: Vec<types::FutureSalt>,

    server_salt_until: i32,
    server_salt: Salt,

    requests: VecDeque<Request<H>>,

    msgs_ack_msg_ids: Vec<MsgId>,
}

impl<T: Transport, H: Handle> Sanity<T, H> {
    #[inline]
    pub(super) fn new(server_msg_ids_capacity: usize, server_salt: Salt) -> Self {
        Self {
            container: None,

            client_msg_ids: ClientMsgIds::new(SystemTime::now()),
            client_seq_nos: ClientSeqNos::new(),

            server_msg_ids: ServerMsgIds::new(server_msg_ids_capacity),
            server_seq_nos: ServerSeqNos::new(),

            get_future_salts_msg: None,

            future_salts: Vec::new(),

            server_salt_until: BAD_SALT_UNTIL,
            server_salt,

            now: None,

            requests: VecDeque::new(),

            msgs_ack_msg_ids: Vec::with_capacity(MAX_IDS_PER_SERVICE_MSG),
        }
    }

    #[inline]
    pub(super) fn take_container(&mut self) -> Option<Container<T>> {
        mem::take(&mut self.container)
    }

    #[inline]
    fn get_container(&mut self, len: usize) -> &mut Container<T> {
        if let Some(ref mut container) = self.container {
            container
        } else {
            let container = self.new_container(len);
            self.container.insert(container)
        }
    }

    // FIXME: create buffer container.
    pub(super) fn new_container(&mut self, len: usize) -> Container<T> {
        Container::new(unbite::DynBuf::new(len + 64 * 1024))
    }

    fn take_buffer(&mut self, capacity: usize) -> unbite::DynRaw {
        // TODO.
        unbite::DynRaw::new(capacity)
    }

    pub(crate) fn push_buffer(&mut self, buffer: unbite::DynRaw) {
        // TODO.
        drop(buffer);
    }

    #[inline]
    pub(super) fn get_msg<const CONTENT_RELATED: bool>(&mut self, system_time: SystemTime) -> Msg {
        let msg_id = self.client_msg_ids.get(system_time);

        let seq_no = if CONTENT_RELATED {
            self.client_seq_nos.get_content_related()
        } else {
            self.client_seq_nos.non_content_related()
        };

        Msg { msg_id, seq_no }
    }

    fn handle_single(
        &mut self,
        buf_msg: BufMsg<'_>,
        _system_time: SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
        handle: &mut H,
    ) -> Result<(), SenderError> {
        use tl::api::types::{
            UpdateShort, UpdateShortChatMessage, UpdateShortMessage, UpdateShortSentMessage,
            Updates, UpdatesCombined, UpdatesTooLong,
        };

        use SenderError::*;

        let BufMsg { msg, mut buf, typ } = buf_msg;

        macro_rules! updates {
            ($typ:ty) => {
                updates.push(buf.de::<$typ>()?.into())
            };
        }

        if let Err(err) = self.server_seq_nos.check(msg.seq_no, typ) {
            match err {
                SeqNoError::Invalid => {
                    warn!("invalid `seq_no`");
                }
                err => return Err(SeqNo(err)),
            }
        }

        self.ack(msg);

        match typ {
            // Even though TDLib implementation is recursive,
            // server should not send these containers nested.
            tl::GZIP_PACKED => return Err(DoubleGzipPacked),
            tl::MSG_CONTAINER => return Err(DoubleMsgContainer),

            tl::RPC_RESULT => self.rpc_result(buf, handle)?,

            types::MsgsAck::CONSTRUCTOR_ID => self.msgs_ack(buf.de()?)?,

            types::BadMsgNotification::CONSTRUCTOR_ID => self.bad_msg_notification(buf.de()?)?,
            types::BadServerSalt::CONSTRUCTOR_ID => self.bad_server_salt(buf.de()?)?,

            types::NewSessionCreated::CONSTRUCTOR_ID => self.new_session_created(buf.de()?)?,
            types::FutureSalts::CONSTRUCTOR_ID => self.handle_future_salts(buf.de()?)?,
            types::Pong::CONSTRUCTOR_ID => self.pong(buf.de()?)?,

            UpdatesTooLong::CONSTRUCTOR_ID => updates.push(UpdatesTooLong {}.into()),

            UpdateShortMessage::CONSTRUCTOR_ID => updates!(UpdateShortMessage),
            UpdateShortChatMessage::CONSTRUCTOR_ID => updates!(UpdateShortChatMessage),
            UpdateShort::CONSTRUCTOR_ID => updates!(UpdateShort),
            UpdatesCombined::CONSTRUCTOR_ID => updates!(UpdatesCombined),
            Updates::CONSTRUCTOR_ID => updates!(Updates),
            UpdateShortSentMessage::CONSTRUCTOR_ID => updates!(UpdateShortSentMessage),

            _ => {
                println!("{typ:#010x}");
                todo!()
            }
        }

        Ok(())
    }

    fn ungzip_packed_bytes<'a>(
        &mut self,
        out: &'a mut Option<unbite::DynBuf>,
        buf: &mut tl::de::Buf<'a>,
    ) -> Result<u32, SenderError> {
        use SenderError::*;

        let bytes = tl::Bytes::deserialize(buf).expect("TODO");
        let bytes = bytes.0.as_slice();

        let gzip_isize = u32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap()) as usize;

        let buffer = out.insert(self.take_buffer(gzip_isize).into_buf());

        buffer
            .try_init_with(|output| Ok(&*ungzip(bytes, output)?))
            .map_err(Ungzip)?;

        *buf = tl::de::Buf::new(buffer.as_slice());

        Ok(buf.de()?)
    }

    pub(super) fn handle(
        &mut self,
        buf_msg: BufMsg<'_>,
        system_time: SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
        handle: &mut H,
    ) -> Result<(), SenderError> {
        use SenderError::*;

        let mut buf_msg = buf_msg;

        self.server_msg_ids.check(buf_msg.msg.msg_id, system_time)?;

        let mut out = None;

        if buf_msg.typ == tl::GZIP_PACKED {
            buf_msg.typ = self.ungzip_packed_bytes(&mut out, &mut buf_msg.buf)?;
        }

        match buf_msg.typ {
            tl::GZIP_PACKED => return Err(DoubleGzipPacked),
            tl::MSG_CONTAINER => {
                let BufMsg { msg, buf, typ } = buf_msg;

                let msg_container = MsgContainerIter::deserialize(buf)
                    .map_err(|err| Deserialization(err.into()))?;

                // Collect first to validate `bytes` field of `message` type.
                let mut container = Vec::with_capacity(msg_container.len());

                for buf_msg in msg_container {
                    let buf_msg = buf_msg?;

                    self.server_msg_ids.check(buf_msg.msg.msg_id, system_time)?;

                    container.push(buf_msg);
                }

                for mut buf_msg in container {
                    let mut out = None;

                    if buf_msg.typ == tl::GZIP_PACKED {
                        buf_msg.typ = self.ungzip_packed_bytes(&mut out, &mut buf_msg.buf)?;
                    }

                    self.handle_single(buf_msg, system_time, updates, handle)?;

                    if let Some(out) = out {
                        self.push_buffer(out.into_raw());
                    }
                }

                if let Err(err) = self.server_seq_nos.check(msg.seq_no, typ) {
                    match err {
                        SeqNoError::Invalid => {
                            warn!("invalid `seq_no`");
                        }
                        err => return Err(SeqNo(err)),
                    }
                }
            }
            _ => self.handle_single(buf_msg, system_time, updates, handle)?,
        }

        if let Some(out) = out {
            self.push_buffer(out.into_raw());
        }

        Ok(())
    }

    fn bad_msg_notification(&mut self, x: types::BadMsgNotification) -> Result<(), SenderError> {
        let types::BadMsgNotification {
            bad_msg_id,
            bad_msg_seqno,
            error_code,
        } = x;

        warn!("received `bad_msg_notification#a7eff811`");

        Ok(())
    }

    fn new_session_created(&mut self, x: types::NewSessionCreated) -> Result<(), SenderError> {
        info!("received `new_session_created#9ec20908`");

        let types::NewSessionCreated {
            first_msg_id: _,
            unique_id: _,
            server_salt,
        } = x;

        self.reset_salts(server_salt);

        Ok(())
    }

    fn pong(&mut self, x: types::Pong) -> Result<(), SenderError> {
        let types::Pong { msg_id, ping_id } = x;

        info!(msg_id, ping_id, "received `pong#347773c5`");

        Ok(())
    }
}
