mod container;
mod error;
mod sanity;

use std::task::{Context, Poll, ready};
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::oneshot;
use tracing::{debug, trace, warn};

use crate::reader::{Reader, ReaderResult};
use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::writer::QueuedWriter;
use crate::{mtproto, tl};

use container::Container;
use sanity::Sanity;

pub use error::SenderError;

struct Request {
    msg: mtproto::Msg,

    tx: oneshot::Sender<tl::Object>,
}

pub enum Messages<'a> {
    Msg(mtproto::BufMsg<'a>),
    MsgContainer(mtproto::Msg, Vec<mtproto::BufMsg<'a>>),
}

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: mtproto::AuthKey,
    session_id: mtproto::Session,

    sanity: Sanity<T>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Sender<T, R, W> {
    pub fn new(
        reader: Reader<R, T>,
        writer: QueuedWriter<W, T>,

        auth_key: mtproto::AuthKey,
        session_id: mtproto::Session,

        salt: mtproto::Salt,
    ) -> Self {
        let mut sanity = Sanity::new(1024, salt);

        sanity.push_get_future_salts();

        Self {
            reader,
            writer,

            auth_key,
            session_id,

            sanity,
        }
    }

    #[expect(clippy::needless_pass_by_ref_mut)]
    fn reserve(&mut self, length: usize) {
        unimplemented!("TODO: reserve(length={length})");
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn push_completed_writer_buffer(&mut self, _buffer: unbite::DynBuf) {
        // warn!("TODO: push_completed_writer_buffer(..)");
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn push_immediate_writer_buffer(&mut self, _buffer: unbite::DynRaw) {
        // warn!("TODO: push_immediate_writer_buffer(..)");
    }

    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        clippy::needless_pass_by_value
    )]
    fn quick_ack(&mut self, quick_ack: QuickAck) {
        warn!("TODO: quick_ack(quick_ack={quick_ack:?})");
    }

    fn get_container(&mut self, len: usize, system_time: SystemTime) -> &mut Container<T> {
        self.sanity.push_msgs_ack();

        if self
            .sanity
            .container
            .as_ref()
            .is_some_and(|c| c.can_push::<false>(len))
        {
            return self.sanity.container.as_mut().unwrap();
        }

        if let Some(container) = self.sanity.take_container() {
            self.queue_container_write(container, system_time);
        }

        let new = self.sanity.new_container(len);

        self.sanity.container.insert(new)
    }

    fn queue_container_write(&mut self, container: Container<T>, system_time: SystemTime) {
        trace!(len = container.len(), "queuing container");

        let (transport, encrypted, buffer) = container.finalize();

        let internal = mtproto::InternalHeader {
            salt: self.sanity.get_salt(),
            session_id: self.session_id,
        };

        let msg = self.sanity.get_msg::<false>(system_time);

        let buffer = self
            .writer
            .queue(transport, encrypted, buffer, &self.auth_key, internal, msg);

        if let Some(buffer) = buffer {
            self.push_immediate_writer_buffer(buffer);
        }
    }

    #[inline]
    fn poll_reader(&mut self, cx: &mut Context<'_>) -> Poll<Result<Packet, SenderError>> {
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
    fn poll_writer(
        &mut self,
        cx: &mut Context<'_>,
        system_time: SystemTime,
    ) -> Result<(), SenderError> {
        if self.poll_writer_once(cx)?.is_pending() {
            return Ok(());
        }

        self.sanity.push_msgs_ack();

        let Some(container) = self.sanity.take_container() else {
            return Ok(());
        };

        self.queue_container_write(container, system_time);

        // We intentionally discard the `Poll<()>` as we do
        // not need the confirmation about writer readiness.
        let _ = self.poll_writer_once(cx)?;

        Ok(())
    }

    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
        updates: &mut Vec<tl::api::enums::Updates>,
    ) -> Poll<Result<(), SenderError>> {
        trace!("polled");

        let system_time = SystemTime::now();

        // Poll the `Reader` first to push ACKs before write.
        if let Poll::Ready(packet) = self.poll_reader(cx)? {
            trace!(data = ?packet.data, "packet");

            self.handle_packet(packet, updates, system_time)?;

            return Poll::Ready(Ok(()));
        }

        self.poll_writer(cx, system_time)?;

        Poll::Pending
    }

    fn handle_packet(
        &mut self,
        packet: Packet,
        updates: &mut Vec<tl::api::enums::Updates>,
        system_time: SystemTime,
    ) -> Result<(), SenderError> {
        use SenderError::*;

        let (internal, mut buf) = self
            .reader
            .encrypted_message(packet, &self.auth_key)
            .map_err(Message)?;

        if internal.session_id != self.session_id {
            return Err(Session(mtproto::SessionIdError(internal.session_id)));
        }

        let buf_msg = mtproto::BufMsg::deserialize(&mut buf)?;

        mtproto::check_random_padding(buf.as_slice()).map_err(Padding)?;

        self.sanity.handle_buf_msg(buf_msg, system_time, updates)?;

        Ok(())
    }

    pub fn invoke<F: FnOnce(&mut tl::ser::Buf)>(
        &mut self,
        len: usize,
        f: F,
    ) -> oneshot::Receiver<tl::Object> {
        debug!(len, "invoking");

        let system_time = SystemTime::now();

        let msg = self.sanity.get_msg::<true>(system_time);

        self.get_container(len, system_time)
            .push::<false, F>(&msg, len, f);

        let (tx, rx) = oneshot::channel();

        let request = Request { msg, tx };

        self.sanity.requests.push_back(request);

        rx
    }
}
