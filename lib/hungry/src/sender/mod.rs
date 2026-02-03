mod container;
mod error;

use std::collections::VecDeque;
use std::mem;
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, trace, warn};

use crate::reader::{Reader, ReaderResult};
use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::unpack::MsgContainerIter;
use crate::writer::QueuedWriter;
use crate::{mtproto, tl};

use tl::SerializedLen;
use tl::mtproto::{enums, types};

use container::Container;

pub use error::SenderError;

pub enum Messages<'a> {
    Msg(mtproto::BufMsg<'a>),
    MsgContainer(mtproto::Msg, Vec<mtproto::BufMsg<'a>>),
}

struct Request {
    msg: mtproto::Msg,

    tx: oneshot::Sender<tl::Object>,
}

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: mtproto::AuthKey,
    session_id: mtproto::Session,

    // FIXME
    salt: mtproto::Salt,

    container: Option<Container<T>>,

    client_msg_ids: mtproto::ClientMsgIds,
    client_seq_nos: mtproto::SeqNos,

    server_msg_ids: mtproto::ServerMsgIds,
    server_seq_nos: mtproto::SeqNos,

    requests: VecDeque<Request>,

    msgs_ack: Vec<mtproto::MsgId>,

    fixme_object_tx: mpsc::UnboundedSender<tl::Object>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Sender<T, R, W> {
    pub fn new(
        reader: Reader<R, T>,
        writer: QueuedWriter<W, T>,

        auth_key: mtproto::AuthKey,
        session_id: mtproto::Session,

        salt: mtproto::Salt,
    ) -> (Self, mpsc::UnboundedReceiver<tl::Object>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let sender = Self {
            reader,
            writer,

            auth_key,
            session_id,

            salt,

            container: None,

            client_msg_ids: mtproto::ClientMsgIds::new(std::time::SystemTime::now()),
            client_seq_nos: mtproto::SeqNos::new(),

            server_msg_ids: mtproto::ServerMsgIds::new(1024),
            server_seq_nos: mtproto::SeqNos::new(),

            requests: VecDeque::new(),
            msgs_ack: Vec::with_capacity(8192),
            fixme_object_tx: tx,
        };

        (sender, rx)
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn reserve(&mut self, length: usize) {
        unimplemented!("TODO: reserve(length={length})");
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn push_completed_writer_buffer(&mut self, _buffer: unbite::DynBuf) {
        warn!("TODO: push_completed_writer_buffer(..)");
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn push_immediate_writer_buffer(&mut self, _buffer: unbite::DynRaw) {
        warn!("TODO: push_immediate_writer_buffer(..)");
    }

    #[inline]
    fn take_container(&mut self) -> Option<Container<T>> {
        mem::take(&mut self.container)
    }

    fn ack(&mut self) {
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
    fn new_container(&mut self, len: usize) -> Container<T> {
        warn!("TODO: new_container(len={len})");

        // FIXME
        Container::new(unbite::DynBuf::new(len + 4096))
    }

    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        clippy::needless_pass_by_value
    )]
    fn quick_ack(&mut self, quick_ack: QuickAck) {
        warn!("TODO: quick_ack(quick_ack={quick_ack:?})");
    }

    #[inline]
    fn get_msg<const CONTENT_RELATED: bool>(&mut self) -> mtproto::Msg {
        let msg_id = self.client_msg_ids.get(std::time::SystemTime::now());

        let seq_no = if CONTENT_RELATED {
            self.client_seq_nos.get_content_related()
        } else {
            self.client_seq_nos.non_content_related()
        };

        mtproto::Msg { msg_id, seq_no }
    }

    fn get_container(&mut self, len: usize) -> &mut Container<T> {
        self.ack();

        if self
            .container
            .as_ref()
            .is_some_and(|c| c.can_push::<false>(len))
        {
            return self.container.as_mut().unwrap();
        }

        if let Some(container) = self.take_container() {
            self.queue_container_write(container);
        }

        let new = self.new_container(len);

        self.container.insert(new)
    }

    fn queue_container_write(&mut self, container: Container<T>) {
        debug!(
            len = container.len(),
            "finalizing `Container` and queuing buffer to the `QueuedWriter`"
        );

        let (transport, encrypted, buffer) = container.finalize();

        let internal = mtproto::InternalHeader {
            salt: self.salt,
            session_id: self.session_id,
        };

        let msg = self.get_msg::<false>();

        let buffer = self
            .writer
            .queue(transport, encrypted, buffer, &self.auth_key, internal, msg);

        if let Some(buffer) = buffer {
            self.push_immediate_writer_buffer(buffer);
        }
    }

    #[inline]
    fn poll_reader<'a>(&'a mut self, cx: &mut Context<'_>) -> Poll<Result<Packet, SenderError>> {
        while let Poll::Ready(result) = self.reader.poll(cx) {
            let unpack = match result {
                ReaderResult::Reserve(length) => {
                    self.reserve(length);

                    continue;
                }
                ReaderResult::Unpack(unpack) => unpack,
                ReaderResult::Error(err) => return Poll::Ready(Err(SenderError::Reader(err))),
            };

            let packet = match unpack {
                Unpack::Packet(packet) => packet,
                Unpack::QuickAck(quick_ack) => {
                    self.quick_ack(quick_ack);

                    cx.waker().wake_by_ref();

                    return Poll::Pending;
                }
            };

            debug!(data = ?packet.data, "packet");

            return Poll::Ready(Ok(packet));
        }

        Poll::Pending
    }

    fn poll_writer_once(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SenderError>> {
        while !self.writer.is_empty() {
            let buffer = ready!(self.writer.poll(cx)).map_err(SenderError::Writer)?;

            self.push_completed_writer_buffer(buffer);
        }

        Poll::Ready(Ok(()))
    }

    #[inline]
    fn poll_writer(&mut self, cx: &mut Context<'_>) -> Result<(), SenderError> {
        if self.poll_writer_once(cx)?.is_pending() {
            return Ok(());
        }

        self.ack();

        let Some(container) = self.take_container() else {
            return Ok(());
        };

        self.queue_container_write(container);

        // We intentionally discard the `Poll<()>` as we do
        // not need the confirmation about writer readiness.
        let _ = self.poll_writer_once(cx)?;

        Ok(())
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SenderError>> {
        trace!("polled");

        self.poll_writer(cx)?;

        let packet = ready!(self.poll_reader(cx))?;

        let messages = self.packet(packet)?;

        match messages {
            Messages::Msg(buf_msg) => match mtproto::Message::deserialize(buf_msg).unwrap() {
                mtproto::Message::Object { msg, obj } => self.handle_object(msg, obj),
                mtproto::Message::RpcResult { msg, res } => self.handle_result(msg, res),
            },
            Messages::MsgContainer(_msg, container) => {
                let mut res = Vec::with_capacity(container.len());

                for buf_msg in container {
                    res.push(mtproto::Message::deserialize(buf_msg).unwrap());
                }

                for res in res {
                    match res {
                        mtproto::Message::Object { msg, obj } => self.handle_object(msg, obj),
                        mtproto::Message::RpcResult { msg, res } => self.handle_result(msg, res),
                    }
                }
            }
        }

        cx.waker().wake_by_ref();

        Poll::Pending
    }

    fn packet(&'_ mut self, packet: Packet) -> Result<Messages<'_>, SenderError> {
        use SenderError::*;

        let (internal, mut buf) = self
            .reader
            .encrypted_message(&packet, &self.auth_key)
            .map_err(Message)?;

        if internal.session_id != self.session_id {
            return Err(Session(mtproto::SessionIdError(internal.session_id)));
        }

        let buf_msg = mtproto::BufMsg::deserialize(&mut buf)?;

        mtproto::check_random_padding(buf.as_slice()).map_err(Padding)?;

        let unix_time = std::time::SystemTime::now();

        if buf_msg.typ != tl::MSG_CONTAINER {
            self.server_msg_ids.check(buf_msg.msg_id, unix_time)?;
            self.server_seq_nos.check_with_typ(&buf_msg)?;

            if buf_msg.seq_no & 1 == 1 {
                self.msgs_ack.push(buf_msg.msg_id);
            }

            return Ok(Messages::Msg(buf_msg));
        }

        debug!("unpacking `msg_container#73f1f8dc`");

        let msg_container =
            MsgContainerIter::new(buf_msg.buf).map_err(|err| Deserialization(err.into()))?;

        let mut container = Vec::with_capacity(msg_container.len());

        for item in msg_container {
            let buf_msg = item?;

            self.server_msg_ids.check(buf_msg.msg_id, unix_time)?;
            self.server_seq_nos.check_with_typ(&buf_msg)?;

            if buf_msg.seq_no & 1 == 1 {
                self.msgs_ack.push(buf_msg.msg_id);
            }

            container.push(buf_msg);
        }

        Ok(Messages::MsgContainer(buf_msg.msg, container))
    }

    pub fn invoke<F: FnOnce(&mut tl::ser::Buf)>(
        &mut self,
        len: usize,
        f: F,
    ) -> oneshot::Receiver<tl::Object> {
        debug!(len, "invoking");

        let msg = self.get_msg::<true>();
        let bytes = len.try_into().unwrap();

        self.get_container(len).push::<false, F>(&msg, bytes, f);

        let (tx, rx) = oneshot::channel();

        let request = Request { msg, tx };

        self.requests.push_back(request);

        rx
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

    fn handle_object(&mut self, _msg: mtproto::Msg, object: tl::Object) {
        use tl::Object::*;

        match object {
            mtproto_Pong(enums::Pong::Pong(ref pong)) => {
                self.send_rpc_result(pong.msg_id, object);
            }
            mtproto_FutureSalts(enums::FutureSalts::FutureSalts(ref future_salts)) => {
                self.send_rpc_result(future_salts.req_msg_id, object);
            }
            mtproto_BadMsgNotification(x) => match x {
                enums::BadMsgNotification::BadMsgNotification(x) => {}
                enums::BadMsgNotification::BadServerSalt(x) => {
                    self.salt = x.new_server_salt;
                }
            },
            object => {
                // Should be an update.
                self.fixme_object_tx.send(object).unwrap();
            }
        }
    }

    fn handle_result(&mut self, _msg: mtproto::Msg, res: mtproto::RpcResult) {
        self.send_rpc_result(res.req_msg_id, res.res_object);
    }
}
