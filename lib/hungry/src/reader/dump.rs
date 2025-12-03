use crate::transport::{Packet, QuickAck, Unpack};
use crate::{mtproto, reader, utils};
use bytes::BytesMut;

pub struct Dump<T: reader::ReaderBehaviour>(pub T);

impl<T: reader::ReaderBehaviour> reader::ReaderBehaviour for Dump<T> {
    type Unpack = T::Unpack;

    fn required(&mut self, buffer: &mut BytesMut, length: usize) {
        println!("READER: required {length}");
        self.0.required(buffer, length);
    }

    fn acquired(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Unpack {
        let title = match &unpack {
            Unpack::Packet(Packet { data, next }) => {
                let message = mtproto::Message::unpack(&buffer[data.clone()]);

                &format!("READER: acquired {message}")
            }
            Unpack::QuickAck(QuickAck { token, len }) => {
                &format!("READER: quick ack 0x{token:08x}, len: {len}")
            }
        };

        utils::dump(buffer, title);

        self.0.acquired(buffer, unpack)
    }
}
