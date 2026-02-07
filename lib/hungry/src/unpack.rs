use std::mem::MaybeUninit;
use std::slice;

use crate::{mtproto, tl};

#[derive(Debug, Eq, PartialEq)]
pub enum UngzipError {
    Inflate(zlib_rs::InflateError),
    StatusOk,
    StatusBufError,
}

pub fn ungzip<'a>(
    input: &[u8],
    output: &'a mut [MaybeUninit<u8>],
) -> Result<&'a mut [u8], UngzipError> {
    use UngzipError::*;

    let mut inflate = zlib_rs::Inflate::new(true, 31);

    let flush = zlib_rs::InflateFlush::Finish;

    // SAFETY: returned slice is up to safe `inflate.total_out()` index.
    let output = unsafe { slice::from_raw_parts_mut(output.as_mut_ptr().cast(), output.len()) };

    let status = inflate.decompress(input, output, flush).map_err(Inflate)?;

    match status {
        zlib_rs::Status::Ok => return Err(StatusOk),
        zlib_rs::Status::BufError => return Err(StatusBufError),
        zlib_rs::Status::StreamEnd => {},
    }

    Ok(&mut output[..inflate.total_out() as usize])
}

pub struct MsgContainerIter<'a> {
    buf: tl::de::Buf<'a>,
    len: u32,
}

impl<'a> MsgContainerIter<'a> {
    /// # Errors
    ///
    /// * [`tl::de::EndOfBufferError`] occurs if the 4-byte `len` read failed.
    pub fn deserialize(mut buf: tl::de::Buf<'a>) -> Result<Self, tl::de::EndOfBufferError> {
        let len = buf.de_infallible::<u32>()?;

        Ok(Self { buf, len })
    }
}

impl<'a> Iterator for MsgContainerIter<'a> {
    type Item = Result<mtproto::BufMsg<'a>, mtproto::BufMsgError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.len = self.len.checked_sub(1)?;

        let buf_msg = match mtproto::BufMsg::deserialize(&mut self.buf) {
            Ok(x) => x,
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
