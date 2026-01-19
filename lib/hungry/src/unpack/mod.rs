use std::fmt;
use std::num::NonZeroUsize;

use crate::{mtproto, tl};

#[derive(Debug, Eq, PartialEq)]
pub enum MsgContainerIterError {
    UnexpectedEndOfBuffer(tl::de::EndOfBufferError),
    NegativeLength,
}

impl fmt::Display for MsgContainerIterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl std::error::Error for MsgContainerIterError {}

struct MsgContainerInner<'a> {
    buf: tl::de::Buf<'a>,
    len: NonZeroUsize,
}

pub struct MsgContainer<'a>(Option<MsgContainerInner<'a>>);

impl<'a> MsgContainer<'a> {
    pub fn new(mut buf: tl::de::Buf<'a>) -> Result<Self, tl::de::EndOfBufferError> {
        let Some(len) = NonZeroUsize::new(u32::from_le_bytes(*buf.take_exactly()?) as usize) else {
            return Ok(Self(None));
        };

        Ok(Self(Some(MsgContainerInner { buf, len })))
    }
}

impl<'a> Iterator for MsgContainer<'a> {
    type Item = Result<(mtproto::Msg, tl::de::Buf<'a>), MsgContainerIterError>;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = self.0.as_mut()?;

        let slice = match inner.buf.take_exactly::<16>() {
            Ok(slice) => slice,
            Err(err) => return Some(Err(MsgContainerIterError::UnexpectedEndOfBuffer(err))),
        };

        let (msg, length) = mtproto::Msg::from_slice(slice);

        let Ok(length) = usize::try_from(length) else {
            return Some(Err(MsgContainerIterError::NegativeLength));
        };

        let slice = match inner.buf.take(length) {
            Ok(slice) => slice,
            Err(err) => return Some(Err(MsgContainerIterError::UnexpectedEndOfBuffer(err))),
        };

        if let Some(len) = NonZeroUsize::new(inner.len.get() - 1) {
            inner.len = len;
        } else {
            self.0 = None;
        }

        let buf = tl::de::Buf::new(slice);

        Some(Ok((msg, buf)))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for MsgContainer<'_> {
    #[inline]
    fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |inner| inner.len.get())
    }
}
