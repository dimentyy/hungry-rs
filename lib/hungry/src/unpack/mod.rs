use crate::{mtproto, tl};

pub struct MsgContainer<'a> {
    buf: tl::de::Buf<'a>,
    len: u32,
}

impl<'a> MsgContainer<'a> {
    /// # Errors
    ///
    /// * [`tl::de::EndOfBufferError`] occurs if the 4-byte `len` read failed.
    pub fn new(mut buf: tl::de::Buf<'a>) -> Result<Self, tl::de::EndOfBufferError> {
        let len = u32::from_le_bytes(*buf.take_exactly()?);

        Ok(Self { buf, len })
    }
}

impl<'a> Iterator for MsgContainer<'a> {
    type Item = Result<mtproto::Msg<tl::de::Buf<'a>>, mtproto::MsgDeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        let msg_de = match mtproto::Msg::deserialize(&mut self.buf) {
            Ok(msg_de) => msg_de,
            Err(err) => return Some(Err(err)),
        };

        Some(Ok(msg_de))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for MsgContainer<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.len as usize
    }
}
