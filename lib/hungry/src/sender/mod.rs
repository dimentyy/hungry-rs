mod container;
mod error;

use std::mem;
use std::ops::ControlFlow;
use std::task::{Context, Poll};

use bytes::BytesMut;

use crate::mtproto::{
    AuthKey, DecryptedMessage, EncryptedMessage, Message, Msg, MsgId, MsgIds, Salt, SeqNos, Session,
};
use crate::reader::{Reader, ReaderDriver};
use crate::tl;
use crate::transport::{Packet, Transport, Unpack};
use crate::writer::{QueuedWriter, WriterDriver};

use container::Container;

pub use error::SenderError;

pub struct Sender<T: Transport, R: ReaderDriver, W: WriterDriver> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: AuthKey,
    salt: Salt,
    session_id: Session,

    container: Container<T>,

    msg_ids: MsgIds,
    seq_nos: SeqNos,
}

impl<T: Transport, R: ReaderDriver, W: WriterDriver> Sender<T, R, W> {
    /// FIXME
    fn new_container() -> Container<T> {
        let buffer = BytesMut::with_capacity(1024 * 1024);

        Container::new(buffer)
    }

    fn get_container(&mut self, len: usize) -> &mut Container<T> {
        if !self.container.can_push(len) {
            let container = Self::new_container();

            let container = mem::replace(&mut self.container, container);

            self.queue(container);
        }

        &mut self.container
    }

    pub fn new(
        reader: Reader<R, T>,
        writer: QueuedWriter<W, T>,
        auth_key: AuthKey,
        salt: Salt,
        session_id: Session,
    ) -> Self {
        Self {
            reader,
            writer,

            container: Self::new_container(),

            msg_ids: MsgIds::new(),
            seq_nos: SeqNos::new(),

            auth_key,
            salt,
            session_id,
        }
    }

    pub fn invoke<X: tl::Function>(
        &mut self,
        func: tl::CalculatedLen<'_, tl::ConstructorId<X>>,
    ) -> MsgId {
        let msg = Msg {
            msg_id: self.msg_ids.get_using_system_time(),
            seq_no: self.seq_nos.get_content_related(),
        };

        let msg_id = msg.msg_id;

        self.get_container(func.len()).push(msg, func);

        msg_id
    }

    fn queue(&mut self, container: Container<T>) {
        let message = DecryptedMessage {
            salt: self.salt,
            session_id: self.session_id,
        };

        let msg = Msg {
            msg_id: self.msg_ids.get_using_system_time(),
            seq_no: self.seq_nos.non_content_related(),
        };

        let (transport, mtp, buffer) = container.finalize();

        let (h, f) = self
            .writer
            .queue(transport, mtp, buffer, &self.auth_key, message, msg);

        if let Some(_h) = h {}

        if let Some(_f) = f {}
    }

    fn unpack(&'_ mut self, unpack: Unpack) -> Result<(), SenderError> {
        let data = match &unpack {
            Unpack::Packet(Packet { data }) => data.clone(),
            Unpack::QuickAck(_) => todo!(),
        };

        let mut buffer = self.reader.buffer().split();

        let buf = &mut buffer[data];

        let encrypted = match Message::unpack(buf) {
            Message::Plain(message) => return Err(SenderError::PlainMessage(message)),
            Message::Encrypted(message) => message,
        };

        let auth_key_id = encrypted.auth_key_id.get();

        if auth_key_id != i64::from_le_bytes(*self.auth_key.id()) {
            return Err(SenderError::UnexpectedAuthKeyId(auth_key_id));
        }

        let buf = &mut buf[EncryptedMessage::HEADER_LEN..];

        let DecryptedMessage { salt, session_id } = encrypted.decrypt(&self.auth_key, buf)?;

        // assert_eq!(decrypted.salt, self.salt);

        if session_id != self.session_id {
            return Err(SenderError::UnexpectedSessionId(session_id));
        }

        // Ok(SenderIter {
        //     sender: self,
        //     buffer,
        //     pos: 0,
        // })

        Ok(())
    }

    pub fn poll<'a>(&'a mut self, cx: &mut Context<'_>) -> Poll<Result<(), SenderError>> {
        if self.writer.is_empty() && !self.container.is_empty() {
            let container = Self::new_container();

            let container = mem::replace(&mut self.container, container);

            self.queue(container);
        }

        loop {
            match self.writer.poll(cx) {
                Poll::Ready(Ok(_buffer)) => {}
                Poll::Ready(Err(err)) => return Poll::Ready(Err(SenderError::Writer(err))),
                Poll::Pending => break,
            }
        }

        loop {
            let unpack = match self.reader.poll(cx) {
                Poll::Ready(ControlFlow::Continue(Ok(unpack))) => unpack,
                Poll::Ready(ControlFlow::Continue(Err(err))) => {
                    return Poll::Ready(Err(SenderError::Reader(err)));
                }
                Poll::Ready(ControlFlow::Break(len)) => {
                    let buf = self.reader.buffer();
                    buf.reserve(len - buf.capacity());
                    continue;
                }
                Poll::Pending => break,
            };

            return Poll::Ready(self.unpack(dbg!(unpack)));
        }

        Poll::Pending
    }
}
