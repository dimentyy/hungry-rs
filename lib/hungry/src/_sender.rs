use std::ops::ControlFlow;
use std::task::{Context, Poll};
use std::{fmt, io};

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::mtproto::{
    AuthKey, DecryptedMessage, EncryptedEnvelope, EncryptedMessage, Message, Msg, MsgId, MsgIds,
    PlainMessage, Salt, SeqNos, Session,
};
use crate::reader::{Error as ReaderError, Reader, ReaderDriver};
use crate::transport::{Packet, Transport, Unpack};
use crate::writer::{QueuedWriter, WriterDriver};
use crate::{Envelope, MsgContainer, mtproto, tl};

pub const MAX_LEN: usize = 1024 * (1024 + 2);


impl<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> Sender<T, R, W> {
    

    

    

    pub fn poll<'a>(
        &'a mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Received<'a, T, R, W>, Error>> {
        
    }
}

pub struct Received<'a, T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    sender: &'a mut Sender<T, R, W>,
    unpack: Unpack,
}

impl<'a, T: Transport, R: ReaderDriver, W: WriterDriver> Received<'a, T, R, W> {
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
            }
        }

        Ok(())
    }

    fn handle(&mut self) -> Result<(), ReceivedError> {
        let data = match &self.unpack {
            Unpack::Packet(Packet { data }) => data.clone(),
            Unpack::QuickAck(_) => unimplemented!(),
        };

        let mut buffer = self.sender.reader.buffer().split();

        let buf = &mut buffer[data];

        let encrypted = match Message::unpack(buf) {
            Message::Plain(message) => return Err(ReceivedError::PlainMessage(message)),
            Message::Encrypted(message) => message,
        };

        let auth_key_id = encrypted.auth_key_id.get();

        if auth_key_id != i64::from_le_bytes(*self.sender.auth_key.id()) {
            return Err(ReceivedError::UnexpectedAuthKeyId(auth_key_id));
        }

        let buf = &mut buf[EncryptedMessage::HEADER_LEN..];

        let DecryptedMessage { salt, session_id } = encrypted.decrypt(&self.sender.auth_key, buf);

        // assert_eq!(decrypted.salt, self.salt);

        if session_id != self.sender.session_id {
            return Err(ReceivedError::UnexpectedSessionId(session_id));
        }

        let mut buf = tl::de::Buf::new(&buf[DecryptedMessage::HEADER_LEN..]);

        let _msg = buf.de::<Msg>()?;

        let bytes = buf.de::<i32>()? as usize;

        self.handle_msg(_msg, buf)?;

        Ok(())
    }
}

impl<'a, T: Transport, R: ReaderDriver, W: WriterDriver> Iterator for Received<'a, T, R, W> {
    type Item = Result<(), ReceivedError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.handle() {
            Ok(_) => {}
            Err(err) => return Some(Err(err)),
        }

        Some(Ok(()))
    }
}
