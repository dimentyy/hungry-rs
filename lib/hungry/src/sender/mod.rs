use std::ops::ControlFlow;
use std::task::{Context, Poll};

use bytes::BytesMut;

use crate::mtproto::{
    AuthKey, DecryptedMessage, EncryptedEnvelope, EncryptedMessage, Message, Msg, MsgId, MsgIds,
    Salt, SeqNos, Session,
};
use crate::reader::{Reader, ReaderDriver};
use crate::transport::{Packet, Transport, Unpack};
use crate::writer::{QueuedWriter, WriterDriver};
use crate::{Envelope, tl};

mod error;

pub use error::{SenderError};

type Container<T> = (crate::MsgContainer, EncryptedEnvelope, Envelope<T>);

pub struct Sender<T: Transport, R: ReaderDriver, W: WriterDriver> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    auth_key: AuthKey,
    salt: Salt,
    session_id: Session,

    container: Option<Container<T>>,

    msg_ids: MsgIds,
    seq_nos: SeqNos,
}

impl<T: Transport, R: ReaderDriver, W: WriterDriver> Sender<T, R, W> {
    fn new_container() -> Container<T> {
        let mut buffer = BytesMut::with_capacity(1024 * 1024);

        let transport = Envelope::split(&mut buffer);
        let mtp = Envelope::split(&mut buffer);

        (crate::MsgContainer::new(buffer), mtp, transport)
    }

    fn get_container(&mut self, capacity: usize) -> &mut crate::MsgContainer {
        let new_required = match &self.container {
            None => false,
            Some((container, _, _)) => container.spare_capacity().map_or(false, |x| x >= capacity),
        };

        if new_required {
            if let Some((container, mtp, transport)) = self.container.take() {
                self.queue(container, mtp, transport);
            }
        }

        &mut self.container.get_or_insert_with(Self::new_container).0
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

            container: None,

            msg_ids: MsgIds::new(),
            seq_nos: SeqNos::new(),

            auth_key,
            salt,
            session_id,
        }
    }

    pub fn invoke<F: tl::Function>(&mut self, func: &F) -> MsgId {
        let len = func.serialized_len();

        let msg = Msg {
            msg_id: self.msg_ids.get_using_system_time(),
            seq_no: self.seq_nos.get_content_related(),
        };

        let msg_id = msg.msg_id;

        self.get_container(len).push(msg, func).unwrap();

        msg_id
    }

    fn queue(
        &mut self,
        container: crate::MsgContainer,
        mtp: EncryptedEnvelope,
        transport: Envelope<T>,
    ) {
        let message = DecryptedMessage {
            salt: self.salt,
            session_id: self.session_id,
        };

        let msg = Msg {
            msg_id: self.msg_ids.get_using_system_time(),
            seq_no: self.seq_nos.non_content_related(),
        };

        let (header, footer) = self.writer.queue(
            container.finalize(),
            transport,
            mtp,
            &self.auth_key,
            message,
            msg,
        );

        if let Some(_header) = header {}

        if let Some(_footer) = footer {}
    }

    fn unpack<'a>(&'a mut self, unpack: Unpack) -> Result<(), SenderError> {
        let data = match &unpack {
            Unpack::Packet(Packet { data }) => data.clone(),
            Unpack::QuickAck(_) => unimplemented!(),
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

        let DecryptedMessage { salt, session_id } = encrypted.decrypt(&self.auth_key, buf);

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

    pub fn poll<'a>(
        &'a mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), SenderError>> {
        if self.writer.is_empty()
            && let Some((container, mtp, transport)) = self.container.take()
        {
            self.queue(container, mtp, transport);
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

            return Poll::Ready(self.unpack(unpack));
        }

        Poll::Pending
    }
}
