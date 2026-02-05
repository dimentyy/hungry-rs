mod container;
mod error;
mod sanity;

use std::task::{Context, Poll, ready};

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

type Result<T = ()> = std::result::Result<T, SenderError>;

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

    fn get_container(&mut self, len: usize) -> &mut Container<T> {
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
            self.queue_container_write(container);
        }

        let new = self.sanity.new_container(len);

        self.sanity.container.insert(new)
    }

    fn queue_container_write(&mut self, container: Container<T>) {
        debug!(
            len = container.len(),
            "finalizing `Container` and queuing buffer to the `QueuedWriter`"
        );

        let (transport, encrypted, buffer) = container.finalize();

        let internal = mtproto::InternalHeader {
            salt: self.sanity.get_salt(),
            session_id: self.session_id,
        };

        let msg = self.sanity.get_msg::<false>();

        let buffer = self
            .writer
            .queue(transport, encrypted, buffer, &self.auth_key, internal, msg);

        if let Some(buffer) = buffer {
            self.push_immediate_writer_buffer(buffer);
        }
    }

    #[inline]
    fn poll_reader(&mut self, cx: &mut Context<'_>) -> Poll<Result<Packet>> {
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

    fn poll_writer_once(&mut self, cx: &mut Context<'_>) -> Poll<Result> {
        while !self.writer.is_empty() {
            let buffer = ready!(self.writer.poll(cx)).map_err(SenderError::Writer)?;

            self.push_completed_writer_buffer(buffer);
        }

        Poll::Ready(Ok(()))
    }

    #[inline]
    fn poll_writer(&mut self, cx: &mut Context<'_>) -> Result {
        if self.poll_writer_once(cx)?.is_pending() {
            return Ok(());
        }

        self.sanity.push_msgs_ack();

        let Some(container) = self.sanity.take_container() else {
            return Ok(());
        };

        self.queue_container_write(container);

        // We intentionally discard the `Poll<()>` as we do
        // not need the confirmation about writer readiness.
        let _ = self.poll_writer_once(cx)?;

        Ok(())
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<Vec<tl::api::enums::Updates>>> {
        trace!("polled");

        // Poll the `Reader` first to push ACKs before write.
        if let Poll::Ready(packet) = self.poll_reader(cx)? {
            let updates = self.handle_packet(packet)?;

            return Poll::Ready(Ok(updates));
        }

        self.poll_writer(cx)?;

        Poll::Pending
    }

    fn handle_packet(&mut self, packet: Packet) -> Result<Vec<tl::api::enums::Updates>> {
        use SenderError::*;

        let unix_time = std::time::SystemTime::now();

        let (internal, mut buf) = self
            .reader
            .encrypted_message(packet, &self.auth_key)
            .map_err(Message)?;

        if internal.session_id != self.session_id {
            return Err(Session(mtproto::SessionIdError(internal.session_id)));
        }

        let buf_msg = mtproto::BufMsg::deserialize(&mut buf)?;

        mtproto::check_random_padding(buf.as_slice()).map_err(Padding)?;

        let mut updates = Vec::new();

        self.sanity
            .handle_buf_msg(buf_msg, unix_time, &mut updates)?;

        Ok(updates)
    }

    pub fn invoke<F: FnOnce(&mut tl::ser::Buf)>(
        &mut self,
        len: usize,
        f: F,
    ) -> oneshot::Receiver<tl::Object> {
        debug!(len, "invoking");

        let msg = self.sanity.get_msg::<true>();

        self.get_container(len).push::<false, F>(&msg, len, f);

        let (tx, rx) = oneshot::channel();

        let request = Request { msg, tx };

        self.sanity.requests.push_back(request);

        rx
    }
}
