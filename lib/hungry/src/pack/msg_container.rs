use crate::mtproto::Msg;

use crate::tl;

use tl::{ConstSerializedLen, Identifiable};

pub struct MsgContainer {
    header: unbite::Raw<8>,
    buffer: unbite::DynBuf,
    length: usize,
}

impl Identifiable for MsgContainer {
    const CONSTRUCTOR_ID: u32 = 0x73f1f8dc;
}

impl MsgContainer {
    const HEADER_LEN: usize = u32::SERIALIZED_LEN + u32::SERIALIZED_LEN;

    #[must_use]
    pub fn new(mut buffer: unbite::DynBuf) -> Self {
        assert!(
            buffer.capacity() >= Self::HEADER_LEN,
            "buffer does not enough capacity"
        );
        assert!(buffer.is_empty(), "buffer is not empty");

        let header = buffer.split_raw_front();

        Self {
            header,
            buffer,
            length: 0,
        }
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    #[must_use]
    pub fn spare_capacity(&self) -> Option<usize> {
        self.buffer
            .spare_capacity_len()
            .checked_sub(Msg::HEADER_LEN)
    }

    #[inline(always)]
    pub fn can_push(&self, len: usize) -> bool {
        self.buffer.spare_capacity_len() >= Msg::HEADER_LEN + len
    }

    pub fn push<X: tl::Function>(&mut self, msg: Msg, x: &X) {
        if !self.can_push(x.serialized_len()) {
            panic!("msg container buffer does not have enough capacity");
        }

        self.buffer.init_with(|spare_capacity| {
            let mut buf = tl::ser::Buf::uninit(spare_capacity);

            buf.ser(&msg);
            buf.ser(&(x.serialized_len() as i32 + 4));
            buf.ser(&X::CONSTRUCTOR_ID);
            buf.ser(x);

            buf.as_slice()
        });

        self.length += 1;
    }

    pub fn finalize(mut self) -> unbite::DynBuf {
        let mut header = self.header.into_buf();

        header.extend_from_array(&Self::CONSTRUCTOR_ID.to_le_bytes());
        header.extend_from_array(&(self.length as u32).to_le_bytes());

        self.buffer.unsplit_buf_front(header);

        self.buffer
    }
}
