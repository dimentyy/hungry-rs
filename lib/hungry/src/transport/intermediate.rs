use crate::transport::{
    Packet, Transport, TransportInit, TransportRead, TransportWrite, Unpack, UnpackResult,
};

#[derive(Default)]
pub struct Intermediate;

pub struct IntermediateRead {}

pub struct IntermediateInit {}

pub struct IntermediateWrite {}

pub struct IntermediateEnvelope {
    header: unbite::Raw<4>,
}

impl Transport for Intermediate {
    type Read = IntermediateRead;
    type Init = IntermediateInit;
    type Write = IntermediateWrite;

    fn split(self) -> (Self::Read, Self::Init, Self::Write) {
        (
            IntermediateRead {},
            IntermediateInit {},
            IntermediateWrite {},
        )
    }

    type Envelope = IntermediateEnvelope;

    fn envelope(buffer: &mut unbite::DynBuf) -> IntermediateEnvelope {
        let header = buffer.split_raw_front();

        IntermediateEnvelope { header }
    }
}

impl TransportRead for IntermediateRead {
    type Transport = Intermediate;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult {
        if buffer.len() < 4 {
            return UnpackResult::Continue { length: 4 };
        }

        let len = i32::from_le_bytes(buffer[0..4].try_into().unwrap()) as usize;

        if len >> 31 == 1 {
            todo!("quick ack")
        }

        if buffer.len() < len + 4 {
            return UnpackResult::Continue { length: len + 4 };
        }

        UnpackResult::Unpacked {
            result: Ok(Unpack::Packet(Packet { data: 4..4 + len })),
            offset: len + 4,
        }
    }
}

impl TransportInit for IntermediateInit {
    type Transport = Intermediate;

    const SIZE: usize = 4;

    #[inline]
    fn init(self, _write: &mut IntermediateWrite, buffer: &mut unbite::DynBuf) {
        buffer.extend_from_array(&[0xee, 0xee, 0xee, 0xee])
    }
}

impl TransportWrite for IntermediateWrite {
    type Transport = Intermediate;

    fn pack(&mut self, buffer: &mut unbite::DynBuf, envelope: IntermediateEnvelope) {
        let mut header = envelope.header.into_buf();

        header.extend_from_array(&buffer.len().to_le_bytes());

        buffer.unsplit_buf_front(header);
    }
}

#[cfg(feature = "obfuscated-transport")]
impl super::IdentifiableTransport for Intermediate {
    const TRANSPORT_IDENTIFIER: [u8; 4] = [0xee, 0xee, 0xee, 0xee];
}
