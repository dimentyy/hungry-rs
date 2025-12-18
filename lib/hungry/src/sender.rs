use bytes::BytesMut;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{fmt, io};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::mtproto::{
    AuthKey, DecryptedMessage, EncryptedEnvelope, Msg, MsgId, MsgIds, Salt, SeqNos, Session,
};
use crate::reader::{Error as ReaderError, Reader};
use crate::transport::{Packet, Transport, Unpack};
use crate::writer::QueuedWriter;
use crate::{Envelope, MsgContainer, mtproto, tl};

pub const MAX_LEN: usize = 1024 * (1024 + 2);

#[derive(Debug)]
pub enum Error {
    Writer(io::Error),
    Reader(ReaderError),
    Deserialization(tl::de::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {}

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,

    buffers: Vec<BytesMut>,

    msg_container: Option<(MsgContainer, EncryptedEnvelope, Envelope<T>)>,

    msg_ids: MsgIds,
    seq_nos: SeqNos,

    auth_key: AuthKey,
    salt: Salt,
    session_id: Session,
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Sender<T, R, W> {
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

            buffers: Vec::new(),
            msg_container: None,

            msg_ids: MsgIds::new(),
            seq_nos: SeqNos::new(),

            auth_key,
            salt,
            session_id,
        }
    }

    fn queue(
        &mut self,
        msg_container: MsgContainer,
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
            msg_container.finalize(),
            transport,
            mtp,
            &self.auth_key,
            message,
            msg,
        );

        if let Some(header) = header {
            self.buffers.push(header)
        }

        if let Some(footer) = footer {
            self.buffers.push(footer)
        }
    }

    fn insert_msg_container(&mut self, mut buffer: BytesMut) {
        let transport = Envelope::split(&mut buffer);
        let mtp = Envelope::split(&mut buffer);

        self.msg_container = Some((MsgContainer::new(buffer), mtp, transport));
    }

    pub fn invoke<F: tl::Function>(&mut self, func: &F) -> MsgId {
        if self.msg_container.is_none() {
            self.insert_msg_container(BytesMut::with_capacity(MAX_LEN));
        }

        let msg = Msg {
            msg_id: self.msg_ids.get_using_system_time(),
            seq_no: self.seq_nos.get_content_related(),
        };

        let msg_id = msg.msg_id;

        self.msg_container
            .as_mut()
            .unwrap()
            .0
            .push(msg, func)
            .unwrap();

        msg_id
    }

    fn handle_container(&mut self, buf: tl::de::Buf<'_>) -> Result<(), tl::de::Error> {
        let container = mtproto::MsgContainer::new(buf)?;

        for message in container {
            let (msg, buf) = message?;
            self.handle_msg(msg, buf)?;
        }

        Ok(())
    }

    fn handle_msg(&mut self, _msg: Msg, mut buf: tl::de::Buf<'_>) -> Result<(), tl::de::Error> {
        let id = buf.de::<u32>()?;

        match id {
            0x73f1f8dc => self.handle_container(buf)?,
            0x9ec20908 => {
                let session = buf.de::<tl::mtproto::types::NewSessionCreated>()?;

                dbg!(session);
            }
            0xf35c6d01 => {
                let req_msg_id = buf.de::<MsgId>()?;

                let id = buf.de::<u32>()?;

                match id {
                    0x2144ca19 => {
                        let err = buf.de::<tl::mtproto::types::RpcError>()?;

                        dbg!(err);
                    }
                    0x8e1a1775 => {
                        let dc = buf.de::<tl::api::types::NearestDc>()?;

                        dbg!(dc);
                    }
                    id => {
                        dbg!(tl::api::types::name(id));
                    }
                }

                dbg!(req_msg_id);
            }
            0xae500895 => {
                let salts = buf.de::<tl::mtproto::types::FutureSalts>()?;

                dbg!(salts);
            }
            0x62d6b459 => {
                let ack = buf.de::<tl::mtproto::types::MsgsAck>()?;

                dbg!(ack);
            }
            0xa7eff811 => {
                let bad = buf.de::<tl::mtproto::types::BadMsgNotification>()?;

                dbg!(bad);
            }
            id => {
                dbg!(tl::mtproto::types::name(id));
            },
        }

        Ok(())
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        if self.writer.is_empty() {
            let (msg_container, mtp, transport) = self.msg_container.take().unwrap();
            if !msg_container.is_empty() {
                self.queue(msg_container, mtp, transport);
            }
            self.insert_msg_container(BytesMut::with_capacity(MAX_LEN));
        }

        loop {
            match self.writer.poll(cx) {
                Poll::Ready(Ok(buffer)) => self.buffers.push(buffer),
                Poll::Ready(Err(err)) => return Poll::Ready(Err(Error::Writer(err))),
                Poll::Pending => break,
            }
        }

        loop {
            let unpack = match self.reader.poll(cx) {
                Poll::Ready(ControlFlow::Continue(Ok(unpack))) => unpack,
                Poll::Ready(ControlFlow::Continue(Err(err))) => {
                    return Poll::Ready(Err(Error::Reader(err)));
                }
                Poll::Ready(ControlFlow::Break(cap)) => {
                    let buf = self.reader.buffer();
                    buf.reserve(cap - buf.capacity());
                    continue;
                }
                Poll::Pending => break,
            };

            let data = match unpack {
                Unpack::Packet(Packet { data }) => data,
                Unpack::QuickAck(_) => unimplemented!(),
            };

            let mut buffer = self.reader.buffer().split();

            let encrypted = match mtproto::Message::unpack(&buffer[data.clone()]) {
                mtproto::Message::Plain(_) => todo!(),
                mtproto::Message::Encrypted(message) => message,
            };

            assert_eq!(
                &encrypted.auth_key_id.get().to_le_bytes(),
                self.auth_key.id()
            );

            let decrypted = encrypted.decrypt(
                &self.auth_key,
                &mut buffer[data.start + mtproto::EncryptedMessage::HEADER_LEN..data.end],
            );

            assert_eq!(decrypted.salt, self.salt);
            assert_eq!(decrypted.session_id, self.session_id);

            let buffer = &buffer[data.start
                + mtproto::EncryptedMessage::HEADER_LEN
                + mtproto::DecryptedMessage::HEADER_LEN..data.end];

            let mut buf = tl::de::Buf::new(buffer);

            let _msg = buf.de::<Msg>().map_err(Error::Deserialization)?;

            let bytes = buf.de::<i32>().map_err(Error::Deserialization)? as usize;

            self.handle_msg(_msg, buf).map_err(Error::Deserialization)?;
        }

        Poll::Pending
    }
}

impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Future for Sender<T, R, W> {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
