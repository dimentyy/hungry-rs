use bytes::BytesMut;

use crate::{reader, transport};

pub struct Split;

impl reader::HandleOutput for Split {
    type Output = (BytesMut, transport::Unpack);

    fn acquired(&mut self, buffer: &mut BytesMut, unpack: transport::Unpack) -> Self::Output {
        (buffer.split(), unpack)
    }
}
