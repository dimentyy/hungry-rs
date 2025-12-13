use bytes::BytesMut;

use crate::reader;

pub struct Reserve;

impl reader::ReserveReaderBuffer for Reserve {
    fn reserve(&mut self, buffer: &mut BytesMut, length: usize) {
        buffer.reserve(buffer.capacity() - length);
    }
}
