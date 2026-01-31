mod container;
mod error;

use std::mem;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::reader::{Reader, ReaderResult};
use crate::transport::{Packet, QuickAck, Transport, Unpack};
use crate::unpack::MsgContainerIter;
use crate::writer::QueuedWriter;
use crate::{mtproto, tl};

use container::Container;

pub use error::SenderError;

pub enum Messages<'a> {
    Msg(mtproto::BufMsg<'a>),
    MsgContainer(mtproto::Msg, Vec<mtproto::BufMsg<'a>>),
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

    server_seq_nos: mtproto::SeqNos,
    server_msg_ids: mtproto::ServerMsgIds,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Sender<T, R, W> {
    pub fn new(
        reader: Reader<R, T>,
        writer: QueuedWriter<W, T>,

        auth_key: mtproto::AuthKey,
        session: mtproto::Session,

        salt: mtproto::Salt,
    ) -> Self {
        Self {
            reader,
            writer,

            auth_key,
            session_id: session,

            salt,

            container: None,

            client_msg_ids: mtproto::ClientMsgIds::new(std::time::SystemTime::now()),
            client_seq_nos: mtproto::SeqNos::new(),

            server_msg_ids: mtproto::ServerMsgIds::new(1024),
            server_seq_nos: mtproto::SeqNos::new(),
        }
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn push_completed_writer_buffer(&mut self, _buffer: unbite::DynBuf) {
        eprintln!("TODO: push_completed_writer_buffer(..)");
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn push_immediate_writer_buffer(&mut self, _buffer: unbite::DynRaw) {
        eprintln!("TODO: push_immediate_writer_buffer(..)");
    }

    #[inline]
    fn take_container(&mut self) -> Option<Container<T>> {
        mem::take(&mut self.container)
    }

    #[expect(clippy::unused_self, clippy::needless_pass_by_ref_mut)]
    fn new_container(&mut self, len: usize) -> Container<T> {
        eprintln!("TODO: new_container(len={len})");

        // FIXME
        Container::new(unbite::DynBuf::new(len + 2048))
    }

    #[expect(
        clippy::unused_self,
        clippy::needless_pass_by_ref_mut,
        clippy::needless_pass_by_value
    )]
    fn quick_ack(&mut self, quick_ack: QuickAck) {
        eprintln!("TODO: quick_ack(quick_ack={quick_ack:?})");
    }

    fn get_container(&mut self, len: usize) -> &mut Container<T> {
        if self.container.as_ref().is_some_and(|c| c.can_push(len)) {
            return self.container.as_mut().unwrap();
        }

        if let Some(container) = self.take_container() {
            self.queue_container_write(container);
        }

        let new = self.new_container(len);

        self.container.insert(new)
    }

    fn queue_container_write(&mut self, container: Container<T>) {
        let (transport, encrypted, buffer) = container.finalize();

        let buffer = self.writer.queue(
            transport,
            encrypted,
            buffer,
            &self.auth_key,
            mtproto::InternalHeader {
                salt: self.salt,
                session_id: self.session_id,
            },
            mtproto::Msg {
                msg_id: self.client_msg_ids.get(std::time::SystemTime::now()),
                seq_no: self.client_seq_nos.non_content_related(),
            },
        );

        if let Some(buffer) = buffer {
            self.push_immediate_writer_buffer(buffer);
        }
    }

    /// # Panics
    ///
    /// * If the provided `len` exceeds the `i32::MAX`.
    pub fn invoke<F: FnOnce(&mut tl::ser::Buf)>(&mut self, len: usize, f: F) -> mtproto::Msg {
        let msg_id = self.client_msg_ids.get(std::time::SystemTime::now());
        let seq_no = self.client_seq_nos.get_content_related();

        let bytes = len.try_into().unwrap();

        let bytes_msg = mtproto::BytesMsg::new(msg_id, seq_no, bytes);

        self.get_container(len).push(&bytes_msg, f);

        bytes_msg.msg
    }

    pub fn poll<'a>(&'a mut self, cx: &mut Context<'_>) -> Poll<Result<Messages<'a>, SenderError>> {
        if !self.writer.is_empty() || self.container.is_some() {
            loop {
                let Poll::Ready(buffer) = self.writer.poll(cx).map_err(SenderError::Writer)? else {
                    let Some(container) = self.take_container() else {
                        break;
                    };

                    self.queue_container_write(container);

                    continue;
                };

                self.push_completed_writer_buffer(buffer);
            }
        }

        if let Poll::Ready(result) = self.reader.poll(cx) {
            let unpack = match result {
                ReaderResult::Reserve(_) => todo!(),
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

            let messages = self.packet(packet)?;

            return Poll::Ready(Ok(messages));
        }

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

        if !mtproto::ENCRYPTED_PADDING_RANGE.contains(&buf.len()) {
            return Err(PaddingLength(buf.len()));
        }

        let unix_time = std::time::SystemTime::now();

        if buf_msg.typ != tl::MSG_CONTAINER {
            self.server_msg_ids.check(buf_msg.msg_id, unix_time)?;
            self.server_seq_nos.check_with_typ(&buf_msg)?;

            return Ok(Messages::Msg(buf_msg));
        }

        let msg_container =
            MsgContainerIter::new(buf_msg.buf).map_err(|err| Deserialization(err.into()))?;

        let mut container = Vec::with_capacity(msg_container.len());

        for item in msg_container {
            let buf_msg = item?;

            self.server_msg_ids.check(buf_msg.msg_id, unix_time)?;
            self.server_seq_nos.check_with_typ(&buf_msg)?;

            container.push(buf_msg);
        }

        Ok(Messages::MsgContainer(buf_msg.msg, container))
    }
}
