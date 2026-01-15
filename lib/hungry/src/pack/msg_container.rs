use std::num::NonZeroU32;

use crate::mtproto::Msg;

use crate::tl;

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
        self.buffer
            .spare_capacity_len()
            .checked_sub(Msg::HEADER_LEN)
    }

    #[inline]
    #[must_use]
    pub fn can_push(&self, len: usize) -> bool {
        len + Msg::HEADER_LEN <= self.buffer.spare_capacity_len().min(i32::MAX as usize)
    }

    #[expect(clippy::needless_pass_by_value)]
    pub fn push<X: tl::Function>(&mut self, msg: Msg, x: &tl::ConstructorId<X>) {
        assert!(
            self.can_push(x.serialized_len()),
            "msg container buffer does not have enough capacity"
        );

        self.buffer.init_with(|spare_capacity| {
            let mut buf = tl::ser::Buf::uninit(spare_capacity);

            buf.ser(&msg);
            buf.ser(&i32::try_from(x.serialized_len()).unwrap());
            buf.ser(x);

            buf.as_slice()
        });

        self.length += 1;
    }

    /// # Panics
    ///
    /// * If no messages were pushed to the container.
    pub fn finalize(mut self) -> MsgContainerResult {
        let length = NonZeroU32::new(self.length).expect("at least one msg in a container");

        if length.get() == 1 {
            return MsgContainerResult::Msg {
                header: self.header,
                buffer: self.buffer,
            };
        }

        let mut header = self.header.into_buf();

        header.extend_from_array(&Self::CONSTRUCTOR_ID.to_le_bytes());
        header.extend_from_array(&self.length.to_le_bytes());

        self.buffer.unsplit_buf_front(header);

        MsgContainerResult::MsgContainer {
            length,
            buffer: self.buffer,
        }
    }
}

#[must_use]
pub enum MsgContainerResult {
    Msg {
        header: unbite::Raw<8>,
        buffer: unbite::DynBuf,
    },
    MsgContainer {
        length: NonZeroU32,
        buffer: unbite::DynBuf,
    },
}
