#![expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    clippy::unused_self
)]

use std::cmp::Reverse;
use std::collections::VecDeque;
use std::mem;
use std::time::{Instant, SystemTime};

use crate::sender::{Container, Request, SenderError};
use crate::transport::Transport;
use crate::unpack::MsgContainerIter;
use crate::{mtproto, tl};

use tracing::{debug, info, warn};

use tl::de::Deserialize;
use tl::mtproto::{enums, funcs, types};
use tl::{Identifiable, SerializedLen};

const BAD_SALT_UNTIL: i32 = i32::MIN;

struct Now {
    unix_time: i32,
    instant: Instant,
}

impl Now {
    #[inline]
    fn new(unix_time: i32) -> Self {
        let instant = Instant::now();

        Self { unix_time, instant }
    }

    #[inline]
    fn unix_time(&self) -> i32 {
        self.unix_time + i32::try_from(self.instant.elapsed().as_secs()).unwrap()
    }
}

// FIXME: this struct manages too many things right now.
pub(super) struct Sanity<T: Transport> {
    pub(super) container: Option<Container<T>>,

    client_msg_ids: mtproto::ClientMsgIds,
    client_seq_nos: mtproto::ClientSeqNos,

    server_msg_ids: mtproto::ServerMsgIds,
    server_seq_nos: mtproto::ServerSeqNos,

    now: Option<Now>,

    get_future_salts_msg: Option<mtproto::Msg>,

    future_salts: Vec<types::FutureSalt>,

    server_salt_until: i32,
    server_salt: mtproto::Salt,

    // FIXME.
    pub(super) requests: VecDeque<Request>,

    msgs_ack_msg_ids: Vec<mtproto::MsgId>,
}

impl<T: Transport> Sanity<T> {
    #[inline]
    pub(super) fn new(server_msg_ids_capacity: usize, server_salt: mtproto::Salt) -> Self {
        Self {
            container: None,

            client_msg_ids: mtproto::ClientMsgIds::new(SystemTime::now()),
            client_seq_nos: mtproto::ClientSeqNos::new(),

            server_msg_ids: mtproto::ServerMsgIds::new(server_msg_ids_capacity),
            server_seq_nos: mtproto::ServerSeqNos::new(),

            get_future_salts_msg: None,
            future_salts: Vec::new(),
            server_salt_until: BAD_SALT_UNTIL,
            server_salt,

            now: None,

            requests: VecDeque::new(),
            msgs_ack_msg_ids: Vec::with_capacity(8192),
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

    pub(super) fn push_get_future_salts(&mut self) {
        // FIXME.
        let num = 1;

        debug!(num, "pushing `get_future_salts#b921bd04`");

        let func = funcs::GetFutureSalts { num };

        let len = func.serialized_len();
        let msg = self.get_msg::<false>(SystemTime::now());

        self.get_container(len)
            .push::<true, _>(&msg, len, |buf| buf.ser(&func));

        self.get_future_salts_msg = Some(msg);
    }

    #[inline]
    pub(super) fn get_msg<const CONTENT_RELATED: bool>(
        &mut self,
        system_time: SystemTime,
    ) -> mtproto::Msg {
        let msg_id = self.client_msg_ids.get(system_time);

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
        _system_time: SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
    ) -> Result<(), SenderError> {
        use SenderError::*;

        let mtproto::BufMsg { msg, mut buf, typ } = buf_msg;

        if let Err(err) = self.server_seq_nos.check(msg.seq_no, typ) {
            match err {
                mtproto::SeqNoError::Invalid => {
                    warn!("invalid `seq_no`");
                }
                err => return Err(SeqNo(err)),
            }
        }

        self.ack(msg);

        match typ {
            tl::GZIP_PACKED => return Err(DoubleGzipPacked),
            tl::MSG_CONTAINER => return Err(DoubleMsgContainer),

            tl::RPC_RESULT => {
                let req_msg_id = buf
                    .de_infallible()
                    .map_err(|err| Deserialization(err.into()))?;

                let typ = buf
                    .de_infallible()
                    .map_err(|err| Deserialization(err.into()))?;

                let object = tl::Object::deserialize(typ, &mut buf)?;

                self.send_rpc_result(req_msg_id, object);
            }

            types::MsgsAck::CONSTRUCTOR_ID => self.msgs_ack(buf.de()?)?,
            types::NewSessionCreated::CONSTRUCTOR_ID => self.new_session_created(buf.de()?)?,
            types::FutureSalts::CONSTRUCTOR_ID => self.handle_future_salts(buf.de()?)?,
            types::Pong::CONSTRUCTOR_ID => self.pong(buf.de()?)?,
            types::BadServerSalt::CONSTRUCTOR_ID => self.bad_server_salt(buf.de()?)?,

            tl::api::types::Updates::CONSTRUCTOR_ID => {
                updates.push(buf.de::<tl::api::types::Updates>()?.into());
            }

            _ => {
                println!("{typ:#010x}");
                todo!()
            }
        }

        Ok(())
    }

    fn ungzip<'a>(
        &mut self,
        out: &'a mut Vec<u8>,
        buf_msg: &mut mtproto::BufMsg<'a>,
    ) -> Result<(), SenderError> {
        let bytes = tl::Bytes::deserialize(&mut buf_msg.buf).expect("TODO");
        let buf = bytes.0.as_slice();

        let gzip_isize = u32::from_le_bytes(buf[buf.len() - 4..].try_into().unwrap());

        *out = vec![0; gzip_isize as usize];

        let config = zlib_rs::InflateConfig { window_bits: 31 };

        let (output, zlib_rs::ReturnCode::Ok) = zlib_rs::decompress_slice(out, buf, config) else {
            todo!()
        };

        buf_msg.buf = tl::de::Buf::new(output);
        buf_msg.typ = buf_msg.buf.de_infallible().expect("TODO");

        Ok(())
    }

    pub(super) fn handle_buf_msg(
        &'_ mut self,
        buf_msg: mtproto::BufMsg<'_>,
        system_time: SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
    ) -> Result<(), SenderError> {
        use SenderError::*;

        let mut buf_msg = buf_msg;

        let mut out = Vec::new();

        self.server_msg_ids.check(buf_msg.msg.msg_id, system_time)?;

        if buf_msg.typ == tl::GZIP_PACKED {
            self.ungzip(&mut out, &mut buf_msg)?;
        }

        match buf_msg.typ {
            tl::GZIP_PACKED => return Err(DoubleGzipPacked),
            tl::MSG_CONTAINER => {
                let mtproto::BufMsg { msg, buf, typ } = buf_msg;

                let msg_container = MsgContainerIter::deserialize(buf)
                    .map_err(|err| Deserialization(err.into()))?;

                // Collect first to validate `bytes` field of `message` type.
                let mut container = Vec::with_capacity(msg_container.len());

                for buf_msg in msg_container {
                    let buf_msg = buf_msg?;

                    self.server_msg_ids.check(buf_msg.msg.msg_id, system_time)?;

                    container.push(buf_msg);
                }

                let mut out = Vec::new();

                for mut buf_msg in container {
                    if buf_msg.typ == tl::GZIP_PACKED {
                        self.ungzip(&mut out, &mut buf_msg)?;
                    }

                    self.handle_single(buf_msg, system_time, updates)?;
                }

                if let Err(err) = self.server_seq_nos.check(msg.seq_no, typ) {
                    match err {
                        mtproto::SeqNoError::Invalid => {
                            warn!("invalid `seq_no`");
                        }
                        err => return Err(SeqNo(err)),
                    }
                }
            }
            _ => self.handle_single(buf_msg, system_time, updates)?,
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

    fn msgs_ack(&mut self, x: types::MsgsAck) -> Result<(), SenderError> {
        let types::MsgsAck { msg_ids } = x;

        debug!(len = msg_ids.len(), "received `msgs_ack#62d6b459`");

        Ok(())
    }

    fn pong(&mut self, x: types::Pong) -> Result<(), SenderError> {
        let types::Pong { msg_id, ping_id } = x;

        info!(msg_id, ping_id, "received `pong#347773c5`");

        Ok(())
    }

    fn bad_server_salt(&mut self, x: types::BadServerSalt) -> Result<(), SenderError> {
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

    fn handle_future_salts(&mut self, x: types::FutureSalts) -> Result<(), SenderError> {
        info!(len = x.salts.0.len(), "received `future_salts#ae500895`");

        if let Some(msg) = self.get_future_salts_msg.take() {
            if msg.msg_id != x.req_msg_id {
                warn!("invalid `req_msg_id`");
            }
        } else {
            warn!("unexpected future salts");
        }

        self.now = Some(Now::new(x.now));

        self.future_salts = x.salts.0;
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

    pub(super) fn get_salt(&mut self) -> mtproto::Salt {
        self.update_current_salt();

        self.server_salt
    }

    #[inline]
    const fn reset_salts(&mut self, server_salt: mtproto::Salt) {
        self.server_salt_until = BAD_SALT_UNTIL;
        self.server_salt = server_salt;
    }

    pub(super) fn push_msgs_ack(&mut self) {
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

    fn ack(&mut self, msg: mtproto::Msg) {
        if !mtproto::is_content_related(msg.seq_no) {
            return;
        }

        let len = self.msgs_ack_msg_ids.len();

        self.msgs_ack_msg_ids.push(msg.msg_id);

        if len < mtproto::MAX_IDS_PER_SERVICE_MSG {
            return;
        }

        self.push_msgs_ack();

        // TODO: handle `BadMsgNotification` to resend the message.
    }
}
