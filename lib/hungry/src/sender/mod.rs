mod container;
mod error;

use std::mem;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::reader::{Reader, ReaderResult};
use crate::transport::{Packet, Transport, Unpack};
use crate::writer::QueuedWriter;
use crate::{mtproto, tl};

use container::Container;

pub use error::SenderError;

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: mtproto::AuthKey,
    session: mtproto::Session,

    // FIXME
    salt: mtproto::Salt,

    container: Option<Container<T>>,

    msg_ids: mtproto::MsgIds,
    seq_nos: mtproto::SeqNos,
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
            session,

            salt,

            container: None,

            msg_ids: mtproto::MsgIds::new(std::time::SystemTime::now()),
            seq_nos: mtproto::SeqNos::new(),
        }
    }

    fn push_completed_writer_buffer(&mut self, _buffer: unbite::DynBuf) {
        eprintln!("TODO: push_completed_writer_buffer(..)");
    }

    fn push_immediate_writer_buffer(&mut self, _buffer: unbite::DynRaw) {
        eprintln!("TODO: push_immediate_writer_buffer(..)");
    }

    fn push_container_header_buffer(&mut self, _buffer: unbite::Raw<8>) {
        eprintln!("TODO: push_container_header_buffer(..)");
    }

    fn take_container(&mut self) -> Option<Container<T>> {
        mem::take(&mut self.container)
    }

    fn new_container(&mut self, len: usize) -> Container<T> {
        eprintln!("TODO: new_container(len={len})");

        // FIXME
        Container::new(unbite::DynBuf::new(len + 100_000))
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
                session_id: self.session,
            },
            mtproto::Msg {
                msg_id: self.msg_ids.get(std::time::SystemTime::now()),
                seq_no: self.seq_nos.non_content_related(),
            },
        );

        if let Some(buffer) = buffer {
            self.push_immediate_writer_buffer(buffer);
        }
    }

    pub fn invoke<F: tl::Function>(&mut self, f: &tl::ConstructorId<F>) {
        let msg = mtproto::Msg {
            msg_id: self.msg_ids.get(std::time::SystemTime::now()),
            seq_no: self.seq_nos.get_content_related(),
        };

        let len = tl::SerializedLen::serialized_len(f);

        self.get_container(len).push(msg, f);
    }

    pub fn poll<'a>(
        &'a mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<tl::de::Buf<'a>, SenderError>> {
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
                Unpack::QuickAck(_) => todo!(),
            };

            let buf = self.packet(packet)?;

            return Poll::Ready(Ok(buf));
        }

        Poll::Pending
    }

    fn packet(&'_ mut self, packet: Packet) -> Result<tl::de::Buf<'_>, SenderError> {
        pub use SenderError::*;

        let buf = self.reader.as_mut_slice(packet);

        if buf.len() < mtproto::ExternalHeader::LEN + mtproto::InternalHeader::LEN {
            return Err(Todo("too small"));
        }

        let (auth_key_id, buf) = buf.split_first_chunk_mut().unwrap();

        let Some(auth_key_id) = mtproto::auth_key_id(*auth_key_id) else {
            return Err(AuthKeyId(mtproto::AuthKeyIdError(None)));
        };

        if auth_key_id != self.auth_key.id() {
            return Err(AuthKeyId(mtproto::AuthKeyIdError(Some(auth_key_id))));
        }

        let (header, buf) = buf.split_first_chunk_mut().unwrap();

        let external = mtproto::ExternalHeader::unpack(auth_key_id, *header);

        let internal = external.decrypt(&self.auth_key, buf).map_err(MsgKeyCheck)?;

        if internal.session_id != self.session {
            return Err(SessionId(mtproto::SessionIdError(internal.session_id)));
        }

        let buf = tl::de::Buf::new(&buf[mtproto::InternalHeader::LEN..]);

        Ok(buf)
    }
}
