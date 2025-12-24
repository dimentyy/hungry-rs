use std::ptr::NonNull;

use bytes::BytesMut;

use crate::mtproto::Msg;
use crate::tl;
use crate::utils::BytesMutExt;

use tl::ser::SerializeUnchecked;
use tl::{ConstSerializedLen, Identifiable};

pub struct MsgContainer {
    header: BytesMut,
    buffer: BytesMut,
    length: usize,
}

impl Identifiable for MsgContainer {
    const CONSTRUCTOR_ID: u32 = 0x73f1f8dc;
}

impl MsgContainer {
    const HEADER_LEN: usize = u32::SERIALIZED_LEN + u32::SERIALIZED_LEN;

    #[must_use]
    pub fn new(mut buffer: BytesMut) -> Self {
        assert!(
            buffer.capacity() >= Self::HEADER_LEN,
            "buffer does not enough capacity"
        );
        assert!(buffer.is_empty(), "buffer is not empty");

        let mut header = buffer.split_left(Self::HEADER_LEN);
        header.clear();

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

    pub fn push<X: tl::Function>(
        &mut self,
        msg: Msg,
        x: tl::CalculatedLen<'_, tl::ConstructorId<X>>,
    ) {
        if !self.can_push(x.len()) {
            panic!("msg container buffer does not have enough capacity");
        }

        unsafe {
            let mut buf = NonNull::new_unchecked(self.buffer.as_mut_ptr().add(self.buffer.len()));

            buf = msg.serialize_unchecked(buf);
            buf = (x.len() as i32).serialize_unchecked(buf);
            x.serialize_unchecked(buf);

            self.buffer
                .set_len(self.buffer.len() + Msg::HEADER_LEN + x.len());
        }

        self.length += 1;
    }

    #[must_use]
    pub fn finalize(mut self) -> BytesMut {
        unsafe {
            let mut buf = NonNull::new_unchecked(self.header.as_mut_ptr());

            buf = Self::CONSTRUCTOR_ID.serialize_unchecked(buf);
            (self.length as u32).serialize_unchecked(buf);

            self.header.set_len(Self::HEADER_LEN);
        }

        self.header.unsplit(self.buffer);

        self.header
    }
}
