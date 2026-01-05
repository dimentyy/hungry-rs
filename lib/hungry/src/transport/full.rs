use std::ops::ControlFlow;

use crate::transport::{
    Packet, Transport, TransportEnvelope, TransportError, TransportInit, TransportRead,
    TransportWrite, Unpack, UnpackResult, bail,
};

#[derive(Default)]
pub struct Full;

pub struct FullRead {
    seq: i32,
}

pub struct FullInit {}

pub struct FullWrite {
    seq: i32,
}

pub struct FullEnvelope {
    header: unbite::Raw<8>,
    footer: unbite::Raw<4>,
}

impl Transport for Full {
    type Read = FullRead;
    type Init = FullInit;
    type Write = FullWrite;

    fn split(self) -> (Self::Read, Self::Init, Self::Write) {
        (FullRead { seq: 0 }, FullInit {}, FullWrite { seq: 0 })
    }
}

impl TransportRead for FullRead {
    type Transport = Full;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult {
        if buffer.len() < 4 {
            return UnpackResult::Continue { length: 4 };
        }

        let len = match i32::from_le_bytes(buffer[0..4].try_into().unwrap()) {
            len @ ..0 => bail!(offset: 4 => Status(-len)),
            len @ 0..12 => bail!(offset: 4 => BadLen(len)),
            len => len as usize,
        };

        if buffer.len() < len {
            return UnpackResult::Continue { length: len };
        }

        let seq = i32::from_le_bytes(buffer[4..8].try_into().unwrap());

        if seq != self.seq {
            bail!(offset: 4 => BadSeq {
                received: seq,
                expected: self.seq,
            });
        }

        let received = u32::from_le_bytes(buffer[len - 4..len].try_into().unwrap());

        let computed = crc32fast::hash(&buffer[0..len - 4]);

        if received != computed {
            bail!(offset: len => BadCrc { received, computed });
        }

        self.seq += 1;

        UnpackResult::Unpacked {
            result: Ok(Unpack::Packet(Packet { data: 8..len - 4 })),
            offset: len,
        }
    }
}

impl TransportInit for FullInit {
    type Transport = Full;

    const SIZE: usize = 0;

    #[inline]
    fn init(self, _write: &mut FullWrite, _buffer: &mut unbite::DynBuf) {}
}

impl TransportWrite for FullWrite {
    type Transport = Full;

    type Envelope = FullEnvelope;

    fn pack(&mut self, buffer: &mut unbite::DynBuf, envelope: Self::Envelope) {
        let mut header = envelope.header.into_buf();

        let len = 4 + 4 + buffer.len() as i32 + 4;

        header.extend_from_array(&len.to_le_bytes());
        header.extend_from_array(&self.seq.to_le_bytes());

        buffer.unsplit_buf_front(header);

        let crc32 = crc32fast::hash(buffer.as_slice());

        buffer.unsplit_raw_back(envelope.footer);

        buffer.extend_from_array(&crc32.to_le_bytes());

        self.seq += 1;
    }
}

impl TransportEnvelope for FullEnvelope {
    fn open(buffer: &mut unbite::DynBuf) -> Self {
        let header = buffer.split_raw_to();
        let footer = buffer.split_raw_off();

        Self { header, footer }
    }
}
