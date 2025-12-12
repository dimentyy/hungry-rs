use bytes::BytesMut;

use crate::mtproto::Msg;
use crate::utils::BytesMutExt;
use crate::{tl, EnvelopeSize};

use tl::ser::Serialize;

pub struct MsgContainer {
    header: BytesMut,
    buffer: BytesMut,
    length: usize,
}

impl EnvelopeSize for MsgContainer {
    const HEADER: usize = 4 + 4;
    const FOOTER: usize = 0;
}

impl MsgContainer {
    #[must_use]
    pub fn new(mut buffer: BytesMut) -> Self {
        assert!(
            buffer.capacity() >= Self::HEADER,
            "buffer does not enough capacity"
        );
        assert!(buffer.is_empty(), "buffer is not empty");

        let header = buffer.split_left(Self::HEADER);

        Self {
            header,
            buffer,
            length: 0,
        }
    }

    pub fn push<X: Serialize>(&mut self, message: Msg, x: &X) -> Result<(), Msg> {
        let len = x.serialized_len();

        if self.buffer.spare_capacity_len() < 16 + len {
            return Err(message);
        }

        self.length += 1;

        message.serialize_into(&mut self.buffer);
        (len as i32).serialize_into(&mut self.buffer);
        x.serialize_into(&mut self.buffer);

        Ok(())
    }

    #[must_use]
    pub fn finalize(mut self) -> BytesMut {
        0x73f1f8dc_u32.serialize_into(&mut self.buffer);
        (self.length as i32).serialize_into(&mut self.buffer);

        self.buffer.unsplit_reverse(self.header);

        self.buffer
    }
}
