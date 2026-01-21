use crate::{mtproto, tl};

use tl::{ConstSerializedLen, Identifiable, SerializedLen};

pub struct MsgContainer {
    header: unbite::Raw<8>,
    buffer: unbite::DynBuf,
    length: u32,
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
        self.length as usize
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    #[must_use]
    pub fn spare_capacity(&self) -> Option<usize> {
        self.buffer.spare_capacity_len().checked_sub(16)
    }

    #[inline]
    #[must_use]
    pub fn can_push(&self, len: usize) -> bool {
        len + 16 <= self.buffer.spare_capacity_len().min(i32::MAX as usize)
    }

    /// # Panics
    ///
    /// * If the internal buffer does not have enough capacity to store `x`.
    pub fn push<X: tl::Function>(&mut self, msg: mtproto::Msg, x: &tl::ConstructorId<X>) {
        assert!(
            self.can_push(x.serialized_len()),
            "msg container buffer does not have enough capacity"
        );

        self.buffer.init_with(|spare_capacity| {
            let mut buf = tl::ser::Buf::uninit(spare_capacity);

            buf.ser(&mtproto::MsgSer::new(msg, x));

            buf.as_slice()
        });

        self.length += 1;
    }

    pub fn finalize(mut self) -> unbite::DynBuf {
        let mut header = self.header.into_buf();

        header.extend_from_array(&Self::CONSTRUCTOR_ID.to_le_bytes());
        header.extend_from_array(&self.length.to_le_bytes());

        self.buffer.unsplit_buf_front(header);

        self.buffer
    }
}
