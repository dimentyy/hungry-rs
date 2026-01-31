use crate::{mtproto, tl};

pub struct MsgContainer {
    header: unbite::Raw<8>,
    buffer: unbite::DynBuf,
    length: u32,

    reserved_messages: u32,
    reserved_capacity: usize,
}

impl MsgContainer {
    const HEADER_LEN: usize = 4 + 4; // CONSTRUCTOR_ID + BareVec

    /// MTProto container can have at most 1024 messages.
    pub const MESSAGES_AT_MOST: u32 = 60;

    /// # Panics
    ///
    /// * If the `buffer` does not have enough capacity to store the header.
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

            reserved_messages: 0,
            reserved_capacity: 0,
        }
    }

    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length as usize
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    #[must_use]
    pub const fn spare_capacity(&self) -> Option<usize> {
        self.buffer.spare_capacity_len().checked_sub(16)
    }

    #[inline]
    #[must_use]
    pub const fn can_push<const RESERVED: bool>(&self, len: usize) -> bool {
        let mut messages = Self::MESSAGES_AT_MOST;
        let mut capacity = self.buffer.spare_capacity_len();

        if const { !RESERVED } {
            messages -= self.reserved_messages;
            capacity -= self.reserved_capacity;
        }

        self.length < messages && len + 16 <= capacity
    }

    /// # Panics
    ///
    /// * If the internal buffer does not have enough capacity to store `x`.
    pub fn push<const RESERVED: bool, F: FnOnce(&mut tl::ser::Buf)>(
        &mut self,
        msg: &mtproto::Msg,
        len: usize,
        f: F,
    ) {
        assert!(self.can_push::<RESERVED>(len));

        self.buffer.init_with(|spare_capacity| {
            let mut buf = tl::ser::Buf::uninit(spare_capacity);

            buf.ser(msg);
            buf.extend_from_array(&i32::try_from(len).unwrap().to_le_bytes());
            f(&mut buf);

            buf.as_slice()
        });

        self.length += 1;
    }

    pub fn finalize(mut self) -> unbite::DynBuf {
        let mut header = self.header.into_buf();

        header.extend_from_array(&tl::MSG_CONTAINER.to_le_bytes());
        header.extend_from_array(&self.length.to_le_bytes());

        self.buffer.unsplit_buf_front(header);

        self.buffer
    }
}
