use crate::{mtproto, tl};

pub struct MsgContainer {
    header: unbite::Raw<8>,
    buffer: unbite::DynBuf,
    length: u32,
}

impl MsgContainer {
    const HEADER_LEN: usize = 4 + 4; // CONSTRUCTOR_ID + BareVec

    pub const MESSAGES_AT_MOST: u32 = 1024;

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
    pub const fn can_push(&self, len: usize) -> bool {
        self.length < Self::MESSAGES_AT_MOST && len + 16 <= self.buffer.spare_capacity_len()
    }

    /// # Panics
    ///
    /// * If the internal buffer does not have enough capacity to store `x`.
    pub fn push<F: FnOnce(&mut tl::ser::Buf)>(&mut self, msg: &mtproto::BytesMsg, f: F) {
        // assert!(
        //     self.can_push(x.serialized_len()),
        //     "msg container buffer does not have enough capacity"
        // );

        self.buffer.init_with(|spare_capacity| {
            let mut buf = tl::ser::Buf::uninit(spare_capacity);

            buf.ser(msg);
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
