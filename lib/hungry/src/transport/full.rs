use crate::transport::{
    Packet, Transport, TransportEnvelope, TransportError, TransportRead, TransportWrite, Unpack,
    UnpackResult, bail,
};

/// # Full
///
/// The basic MTProto transport protocol
///
/// * Overhead: medium
/// * Minimum envelope length: 12 bytes (length+seqno+crc)
/// * Maximum envelope length: 12 bytes (length+seqno+crc)
///
/// Payload structure:
///
/// ```text
/// +----+----+----...----+----+
/// |len.|seq.|  payload  |crc.|
/// +----+----+----...----+----+
/// ```
///
/// Envelope description:
///
/// * Length: length+seqno+payload+crc length encoded as 4 length bytes
///   (little endian, the length of the length field must be included, too)
///
/// * Seqno: the TCP sequence number for this TCP connection
///   (different from the [MTProto sequence number]): the first
///   packet sent is numbered 0, the next one 1, etc.
///
/// * Payload: MTProto payload
///
/// * Crc: 4 CRC32 bytes computed using length,
///   sequence number, and payload together.
///
/// ---
///
/// <https://core.telegram.org/mtproto/mtproto-transports#full>
///
/// [MTProto sequence number]: https://core.telegram.org/mtproto/description#message-sequence-number-msg-seqno
#[derive(Default)]
pub struct Full;

pub struct FullRead {
    seq: i32,
}

pub struct FullWrite {
    seq: i32,
}

pub struct FullEnvelope {
    header: unbite::Raw<8>,
    footer: unbite::Raw<4>,
}

impl crate::Sealed for Full {}

impl Transport for Full {
    type Read = FullRead;
    type Write = FullWrite;

    const INIT_SIZE: usize = 0;

    #[inline]
    fn init(self, _writer_buffer: &mut unbite::DynBuf) -> (Self::Read, Self::Write) {
        (FullRead { seq: 0 }, FullWrite { seq: 0 })
    }

    type Envelope = FullEnvelope;

    fn envelope(buffer: &mut unbite::DynBuf) -> FullEnvelope {
        let header = buffer.split_raw_front();
        let footer = buffer.split_raw_back();

        FullEnvelope { header, footer }
    }
}

impl TransportRead for FullRead {
    type Transport = Full;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult {
        if buffer.len() < 4 {
            return UnpackResult::Continue { length: 4 };
        }

        let got = i32::from_le_bytes(buffer[0..4].try_into().unwrap());

        let Ok(len) = usize::try_from(got) else {
            bail!(offset: 4 => Status(-got));
        };

        if len < 12 {
            bail!(offset: 4 => BadLen(got));
        }

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

impl TransportWrite for FullWrite {
    type Transport = Full;

    /// # Panics
    ///
    /// * If the `buffer.len() + 12` exceeds the [`i32::MAX`].
    fn pack(&mut self, buffer: &mut unbite::DynBuf, envelope: FullEnvelope) {
        let mut header = envelope.header.into_buf();

        let len = 4 + 4 + buffer.len() + 4;

        let len: i32 = len.try_into().unwrap();

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
    #[inline]
    fn header_swap<const N: usize>(&mut self, buffer: &mut unbite::Raw<N>) {
        self.header.swap(buffer);
    }
}
