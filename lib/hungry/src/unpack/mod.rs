use crate::{mtproto, tl};

pub struct MsgContainerIter<'a> {
    buf: tl::de::Buf<'a>,
    len: u32,
}

impl<'a> MsgContainerIter<'a> {
    /// # Errors
    ///
    /// * [`tl::de::EndOfBufferError`] occurs if the 4-byte `len` read failed.
    pub fn new(mut buf: tl::de::Buf<'a>) -> Result<Self, tl::de::EndOfBufferError> {
        let len = u32::from_le_bytes(*buf.take_exactly()?);

        Ok(Self { buf, len })
    }
}

impl<'a> Iterator for MsgContainerIter<'a> {
    type Item = Result<mtproto::BufMsg<'a>, mtproto::BufMsgError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        let buf_msg = match mtproto::BufMsg::deserialize(&mut self.buf) {
            Ok(buf_msg) => buf_msg,
            Err(err) => return Some(Err(err)),
        };

        Some(Ok(buf_msg))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for MsgContainerIter<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.len as usize
    }
}
