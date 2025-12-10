use std::ops::RangeFrom;

use bytes::BytesMut;

use crate::transport::{err, Error, Packet, Transport, TransportRead, TransportWrite, Unpack};
use crate::utils::SliceExt;
use crate::{crypto, Envelope, EnvelopeSize};

#[derive(Default)]
pub struct Full;

pub struct FullRead {
    seq: i32,
}

pub struct FullWrite {
    seq: i32,
}

impl Transport for Full {
    type Read = FullRead;
    type Write = FullWrite;

    fn split(self) -> (Self::Read, Self::Write) {
        (FullRead { seq: 0 }, FullWrite { seq: 0 })
    }
}

impl EnvelopeSize for Full {
    const HEADER: usize = 8;
    const FOOTER: usize = 4;
}

impl TransportRead for FullRead {
    type Transport = Full;

    fn length(&mut self, buffer: &mut [u8]) -> usize {
        match i32::from_le_bytes(*buffer[0..4].arr()) {
            ..0 => 4,
            0..12 => 4,
            len => len as usize,
        }
    }

    fn unpack(&mut self, buffer: &mut [u8]) -> Result<Unpack, Error> {
        let len = match i32::from_le_bytes(*buffer[0..4].arr()) {
            len @ ..0 => err!(Status(-len)),
            len @ 0..12 => err!(BadLen(len)),
            len => len as usize,
        };

        assert!(buffer.len() >= len);

        let seq = i32::from_le_bytes(*buffer[4..8].arr());

        if seq != self.seq {
            err!(BadSeq {
                received: seq,
                expected: self.seq,
            });
        }

        let received = u32::from_le_bytes(*buffer[len - 4..len].arr());

        let computed = crypto::crc32!(&buffer[0..len - 4]);

        if received != computed {
            err!(BadCrc { received, computed })
        }

        self.seq += 1;

        Ok(Unpack::Packet(Packet {
            data: 8..len - 4,
            next: len,
        }))
    }
}

impl TransportWrite for FullWrite {
    type Transport = Full;

    fn pack(
        &mut self,
        buffer: &mut BytesMut,
        mut envelope: Envelope<Self::Transport>,
    ) -> RangeFrom<usize> {
        let excess = envelope.adapt(buffer);
        let (h, f) = envelope.buffers();

        let len = 4 + 4 + buffer.len() as i32 + 4;

        h[0..4].copy_from_slice(&len.to_le_bytes());
        h[4..8].copy_from_slice(&self.seq.to_le_bytes());

        let crc = crypto::crc32!(h, buffer);

        f[0..4].copy_from_slice(&crc.to_le_bytes());

        self.seq += 1;

        envelope.unsplit(buffer, excess);

        0..
    }
}
