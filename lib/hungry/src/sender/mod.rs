#![forbid(unsafe_code)]
#![expect(
    clippy::needless_pass_by_ref_mut,
    clippy::unused_self,
    clippy::needless_pass_by_value,
    clippy::unnecessary_wraps,
    reason = "TODO"
)]

mod container;
mod error;
mod sanity;

use std::task::{Context, Poll, ready};
use std::time::SystemTime;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, trace, warn};

use crate::mtproto::{
    AuthKey, BufMsg, InternalHeader, Msg, Salt, SessionId, SessionIdError, check_random_padding,
};
use crate::reader::{Reader, ReaderResult};
use crate::tl;
use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::writer::QueuedWriter;

use container::Container;
use sanity::Sanity;

pub use error::SenderError;
pub use sanity::Handle;

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin, H: Handle> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: AuthKey,
    session_id: SessionId,

    sanity: Sanity<T, H>,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin, H: Handle> Sender<T, R, W, H> {
    pub fn new(
        reader: Reader<R, T>,
        writer: QueuedWriter<W, T>,

        auth_key: AuthKey,
        session_id: SessionId,

        salt: Salt,
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

    fn reserve(&mut self, length: usize) {
        unimplemented!("TODO: reserve(length={length})");
    }

    fn push_completed_writer_buffer(&mut self, buffer: unbite::DynBuf) {
        self.sanity.push_buffer(buffer.into_raw());
    }

    fn push_immediate_writer_buffer(&mut self, buffer: unbite::DynRaw) {
        self.sanity.push_buffer(buffer);
    }

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

        let (transport, encrypted, buffer, msgs) = container.finalize();

        let internal = InternalHeader {
            salt: self.sanity.get_salt(),
            session_id: self.session_id,
        };

        let msg = self.sanity.get_msg::<false>(system_time);

        self.sanity.container_msgs(msg, msgs);
        
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
        handle: &mut H,
    ) -> Poll<Result<(), SenderError>> {
        trace!("polled");

        let system_time = SystemTime::now();

        // Poll the `Reader` first to push ACKs before write.
        if let Poll::Ready(packet) = self.poll_reader(cx)? {
            trace!(data = ?packet.data, "packet");

            self.handle_packet(packet, system_time, updates, handle)?;

            return Poll::Ready(Ok(()));
        }

        self.poll_writer(cx, system_time)?;

        Poll::Pending
    }

    fn handle_packet(
        &mut self,
        packet: Packet,
        system_time: SystemTime,
        updates: &mut Vec<tl::api::enums::Updates>,
        handle: &mut H,
    ) -> Result<(), SenderError> {
        use SenderError::*;

        let (internal, mut buf) = self
            .reader
            .encrypted_message(packet, &self.auth_key)
            .map_err(Message)?;

        if internal.session_id != self.session_id {
            return Err(Session(SessionIdError(internal.session_id)));
        }

        let buf_msg = BufMsg::deserialize(&mut buf)?;

        check_random_padding(buf.as_slice()).map_err(Padding)?;

        self.sanity.handle(buf_msg, system_time, updates, handle)?;

        Ok(())
    }

    pub fn invoke<F: FnOnce(&mut tl::ser::Buf)>(
        &mut self,
        extra: H::Extra,
        len: usize,
        f: F,
    ) -> Msg {
        debug!(len, "invoking");

        let system_time = SystemTime::now();

        let msg = self.sanity.get_msg::<true>(system_time);

        self.get_container(len, system_time)
            .push::<false, F>(msg, len, f);

        self.sanity.rpc_request(msg, extra);

        msg
    }
}
