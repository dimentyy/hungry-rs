use bytes::BytesMut;

use crate::transport::{Packet, QuickAck, Unpack};
use crate::{mtproto, reader, utils};

pub struct Dump<T: reader::Handle>(pub T);

impl<T: reader::Handle> reader::HandleOutput for Dump<T> {
    type Output = T::Output;

    fn acquired(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Output {
        let title = match &unpack {
            Unpack::Packet(Packet { data, next }) => {
                let message = mtproto::Message::unpack(&buffer[data.clone()]);

                &format!("READER: acquired {message}")
            }
            Unpack::QuickAck(QuickAck { token, len }) => {
                &format!("READER: acquired quick ack 0x{token:08x}, len: {len}")
            }
        };

        utils::dump(buffer, title);

        self.0.acquired(buffer, unpack)
    }
}

impl<T: reader::Handle> reader::HandleBuffer for Dump<T> {
    fn required(&mut self, buffer: &mut BytesMut, length: usize) {
        println!("READER: required {length}");

        self.0.required(buffer, length);
    }
}
