use bytes::BytesMut;

use crate::transport::{Packet, QuickAck, Unpack};
use crate::{mtproto, reader, utils};

pub struct Dump<T: reader::HandleReader>(pub T);

impl<T: reader::HandleReader> reader::ProcessReaderPacket for Dump<T> {
    type Output = T::Output;

    fn process(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Output {
        let title = match &unpack {
            Unpack::Packet(Packet { data }) => {
                let message = mtproto::Message::unpack(&buffer[data.clone()]);

                &format!("READER: acquired {message}")
            }
            Unpack::QuickAck(QuickAck { token, len }) => {
                &format!("READER: acquired quick ack 0x{token:08x}, len: {len}")
            }
        };

        utils::dump(buffer, title).unwrap();

        self.0.process(buffer, unpack)
    }
}

impl<T: reader::HandleReader> reader::ReserveReaderBuffer for Dump<T> {
    fn reserve(&mut self, buffer: &mut BytesMut, length: usize) {
        println!("READER: required {length}");

        self.0.reserve(buffer, length);
    }
}
