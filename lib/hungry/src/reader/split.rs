use bytes::BytesMut;

use crate::reader;
use crate::transport::Unpack;

pub struct Split;

impl reader::ReaderBehaviour for Split {
    type Unpack = (BytesMut, Unpack);

    fn required(&mut self, buffer: &mut BytesMut, length: usize) {
        buffer.reserve(buffer.capacity() - length);
    }

    fn acquired(&mut self, buffer: &mut BytesMut, unpack: Unpack) -> Self::Unpack {
        (buffer.split(), unpack)
    }
}
