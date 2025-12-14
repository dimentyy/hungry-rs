use bytes::BytesMut;

use crate::mtproto::Msg;
use crate::tl;
use crate::utils::BytesMutExt;

use tl::ser::{SerializeInto, SerializeUnchecked};

pub struct MsgContainer {
    header: BytesMut,
    buffer: BytesMut,
    length: usize,
}

impl MsgContainer {
    #[must_use]
    pub fn new(mut buffer: BytesMut) -> Self {
        assert!(buffer.capacity() >= 8, "buffer does not enough capacity");
        assert!(buffer.is_empty(), "buffer is not empty");

        let header = buffer.split_left(8);

        Self {
            header,
            buffer,
            length: 0,
        }
    }

    pub fn push<X: SerializeUnchecked>(&mut self, message: Msg, x: &X) -> Result<(), Msg> {
        let len = x.serialized_len();

        if self.buffer.spare_capacity_len() < 16 + len {
            return Err(message);
        }

        self.length += 1;

        self.buffer.ser(&message);
        self.buffer.ser(&(len as i32));
        self.buffer.ser(x);

        Ok(())
    }

    #[must_use]
    pub fn finalize(mut self) -> BytesMut {
        self.buffer.ser(&0x73f1f8dc_u32);
        self.buffer.ser(&(self.length as i32));

        self.buffer.unsplit_reverse(self.header);

        self.buffer
    }
}
