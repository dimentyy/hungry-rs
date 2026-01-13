mod container;

use std::mem;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::reader::{Reader, ReaderError, ReaderResult};
use crate::transport::{Transport, Unpack};
use crate::writer::{QueuedWriter, WriterError};
use crate::{mtproto, tl};

use container::Container;

#[derive(Debug)]
pub enum SenderError {
    Reader(ReaderError),
    Writer(WriterError),
}

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: mtproto::AuthKey,
    session: mtproto::Session,

    // FIXME
    salt: mtproto::Salt,

    container: Container<T>,

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

            // FIXME
            container: Container::new(unbite::DynBuf::new(100_000)),

            msg_ids: mtproto::MsgIds::new(std::time::SystemTime::now()),
            seq_nos: mtproto::SeqNos::new(),
        }
    }

    fn push_completed_writer_buffer(&mut self, _buffer: unbite::DynBuf) {
        eprintln!("TODO: push_completed_writer_buffer")
    }

    fn push_immediate_writer_buffer(&mut self, _buffer: unbite::DynRaw) {
        eprintln!("TODO: push_immediate_writer_buffer")
    }

    // FIXME
    fn take_container(&mut self) -> Container<T> {
        mem::replace(
            &mut self.container,
            Container::new(unbite::DynBuf::new(100_000)),
        )
    }

    fn queue_container_write(&mut self, container: Container<T>) {
        let (envelope, header, pad, buffer) = container.finalize();

        let buffer = self.writer.queue(
            envelope,
            header,
            buffer,
            pad,
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
        self.container.push(
            mtproto::Msg {
                msg_id: self.msg_ids.get(std::time::SystemTime::now()),
                seq_no: self.seq_nos.get_content_related(),
            },
            f,
        );
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SenderError>> {
        if !self.writer.is_empty() || !self.container.is_empty() {
            loop {
                let Poll::Ready(buffer) = self.writer.poll(cx).map_err(SenderError::Writer)? else {
                    if self.container.is_empty() {
                        break;
                    }

                    let container = self.take_container();

                    self.queue_container_write(container);

                    continue;
                };

                self.push_completed_writer_buffer(buffer);
            }
        }

        while let Poll::Ready(result) = self.reader.poll(cx) {
            let unpack = match result {
                ReaderResult::Reserve(_) => todo!(),
                ReaderResult::Unpack(unpack) => unpack,
                ReaderResult::Error(err) => return Poll::Ready(Err(SenderError::Reader(err))),
            };

            let packet = match unpack {
                Unpack::Packet(packet) => packet,
                Unpack::QuickAck(_) => todo!(),
            };

            let buf = &mut self.reader.buffer().as_mut_slice()[packet.data];

            if buf.len() < mtproto::PlainMsgHeader::LEN {
                todo!()
            }

            let (auth_key_id, buf) = buf.split_first_chunk_mut().unwrap();

            let Some(auth_key_id) = mtproto::auth_key_id(*auth_key_id) else {
                todo!()
            };

            if auth_key_id != self.auth_key.id() {
                todo!()
            }

            let (header, buf) = buf.split_first_chunk_mut().unwrap();

            let external = mtproto::ExternalHeader::unpack(auth_key_id, *header);

            let internal = external.decrypt(&self.auth_key, buf).expect("todo");

            if internal.session_id != self.session {
                todo!()
            }

            let mut buf = tl::de::Buf::new(&buf[mtproto::InternalHeader::LEN..]);

            let msg = dbg!(buf.de::<mtproto::Msg>().expect("todo"));

            if !mtproto::is_msg_id_valid(msg.msg_id, std::time::SystemTime::now()) {
                todo!()
            }

            // TODO: check seq no

            let len = buf.de::<i32>().expect("todo");

            let id = buf.de::<u32>().expect("todo");

            println!("{id:#010x}");
        }

        Poll::Pending
    }
}
