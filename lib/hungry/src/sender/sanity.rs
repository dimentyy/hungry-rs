use std::cmp::Reverse;
use std::collections::VecDeque;
use std::mem;
use std::time::Instant;

use crate::sender::{Container, Request, Result, SenderError};
use crate::transport::Transport;
use crate::unpack::MsgContainerIter;
use crate::{mtproto, tl};

use tracing::{debug, info, warn};

use tl::de::Deserialize;
use tl::mtproto::{enums, funcs, types};
use tl::{Identifiable, SerializedLen};

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

pub(super) struct Sanity<T: Transport> {
    pub(super) container: Option<Container<T>>,

    pub(super) client_msg_ids: mtproto::ClientMsgIds,
    pub(super) client_seq_nos: mtproto::SeqNos,

    pub(super) server_msg_ids: mtproto::ServerMsgIds,
    pub(super) server_seq_nos: mtproto::SeqNos,

    now: Option<Now>,

    salts_req_id: Option<mtproto::MsgId>,
    future_salts: Vec<types::FutureSalt>,
    current_salt: types::FutureSalt,

    pub(super) requests: VecDeque<Request>,

    pub(super) msgs_ack: Vec<mtproto::MsgId>,
}

impl<T: Transport> Sanity<T> {
    #[inline]
    pub(super) fn new(server_msg_ids_capacity: usize, server_salt: mtproto::Salt) -> Self {
        Self {
            container: None,

            client_msg_ids: mtproto::ClientMsgIds::new(std::time::SystemTime::now()),
            client_seq_nos: mtproto::SeqNos::new(),

            server_msg_ids: mtproto::ServerMsgIds::new(server_msg_ids_capacity),
            server_seq_nos: mtproto::SeqNos::new(),

            salts_req_id: None,
            future_salts: Vec::new(),
            current_salt: types::FutureSalt {
                valid_since: 0,
                valid_until: 0,
                salt: server_salt,
            },

            now: None,

            requests: VecDeque::new(),
            msgs_ack: Vec::with_capacity(8192),
        }
    }

    #[inline]
    pub(super) fn take_container(&mut self) -> Option<Container<T>> {
        mem::take(&mut self.container)
    }

    pub(super) fn push_msgs_ack(&mut self) {
        if self.msgs_ack.is_empty() {
            return;
        }

        let msg_ids = mem::take(&mut self.msgs_ack);

        debug!(len = msg_ids.len(), "pushing `msgs_ack#62d6b459`");

        let func: enums::MsgsAck = types::MsgsAck { msg_ids }.into();

        let len = func.serialized_len();
        let msg = self.get_msg::<false>();

        let container = if let Some(ref mut container) = self.container {
            container
        } else {
            let container = self.new_container(len);
            self.container.insert(container)
        };

        container.push::<true, _>(&msg, len, |buf| buf.ser(&func));

        let enums::MsgsAck::MsgsAck(types::MsgsAck { msg_ids }) = func;

        self.msgs_ack = msg_ids;
        self.msgs_ack.clear();
    }

    pub(super) fn push_get_future_salts(&mut self) {
        if self.salts_req_id.is_some() {
            return;
        }

        // FIXME.
        let num = 1;

        debug!(num, "pushing `get_future_salts#b921bd04`");

        let func = funcs::GetFutureSalts { num };

        let len = func.serialized_len();
        let msg = self.get_msg::<false>();

        let container = if let Some(ref mut container) = self.container {
            container
        } else {
            let container = self.new_container(len);
            self.container.insert(container)
        };

        container.push::<true, _>(&msg, len, |buf| buf.ser(&func));

        self.salts_req_id = Some(msg.msg_id);
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
        _unix_time: std::time::SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
    ) -> Result {
        use SenderError::*;

        let mtproto::BufMsg { msg, mut buf, typ } = buf_msg;

        let content_related = self.server_seq_nos.check(msg.seq_no, typ)?;

        if content_related {
            self.msgs_ack.push(msg.msg_id);
        }

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
            types::MsgsAck::CONSTRUCTOR_ID => {
                let msgs_ack: types::MsgsAck = buf.de()?;

                dbg!(msgs_ack);
            }
            types::NewSessionCreated::CONSTRUCTOR_ID => {
                self.handle_new_session_created(buf.de()?)?;
            }
            types::FutureSalts::CONSTRUCTOR_ID => {
                self.handle_future_salts(buf.de()?)?;
            }
            types::Pong::CONSTRUCTOR_ID => {
                let pong: types::Pong = buf.de()?;

                dbg!(pong);
            }
            types::BadServerSalt::CONSTRUCTOR_ID => {
                self.handle_bad_server_salt(buf.de()?)?;
            }

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

    #[expect(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
    fn ungzip<'a>(&mut self, out: &'a mut Vec<u8>, buf_msg: &mut mtproto::BufMsg<'a>) -> Result {
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
        unix_time: std::time::SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
    ) -> Result {
        use SenderError::*;

        let mut buf_msg = buf_msg;

        let mut out = Vec::new();

        self.server_msg_ids.check(buf_msg.msg_id, unix_time)?;

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
                    let buf_msg = buf_msg?;

                    self.server_msg_ids.check(buf_msg.msg_id, unix_time)?;

                    container.push(buf_msg);
                }

                let mut out = Vec::new();

                for mut buf_msg in container {
                    if buf_msg.typ == tl::GZIP_PACKED {
                        self.ungzip(&mut out, &mut buf_msg)?;
                    }

                    self.handle_single(buf_msg, unix_time, updates)?;
                }

                let false = self.server_seq_nos.check(msg.seq_no, typ)? else {
                    unreachable!();
                };
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

    #[expect(clippy::needless_pass_by_value)]
    fn handle_new_session_created(&mut self, x: types::NewSessionCreated) -> Result {
        info!("received `new_session_created#9ec20908`");

        self.current_salt = types::FutureSalt {
            valid_since: 0,
            valid_until: 0,
            salt: x.server_salt,
        };

        Ok(())
    }

    #[expect(clippy::needless_pass_by_value)]
    fn handle_bad_server_salt(&mut self, x: types::BadServerSalt) -> Result {
        info!("received `bad_server_salt#edab447b`");

        if let Some(salts_req_id) = self.salts_req_id.take()
            && salts_req_id == x.bad_msg_id
        {
            self.salts_req_id = None;
            self.push_get_future_salts();
        }

        self.current_salt = types::FutureSalt {
            valid_since: 0,
            valid_until: 0,
            salt: x.new_server_salt,
        };

        Ok(())
    }

    fn handle_future_salts(&mut self, x: types::FutureSalts) -> Result {
        info!(len = x.salts.0.len(), "received `future_salts#ae500895`");

        if let Some(salts_req_id) = self.salts_req_id.take() {
            if salts_req_id != x.req_msg_id {
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

        while self.current_salt.valid_until < unix_time {
            let Some(current_salt) = self.future_salts.pop() else {
                break;
            };

            self.current_salt = current_salt;
        }

        if self.future_salts.is_empty() {
            self.push_get_future_salts();
        }
    }

    pub(super) fn get_salt(&mut self) -> mtproto::Salt {
        self.update_current_salt();

        self.current_salt.salt
    }
}
