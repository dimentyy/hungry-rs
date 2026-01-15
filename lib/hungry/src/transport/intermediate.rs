use crate::transport::{
    Packet, QuickAck, Transport, TransportEnvelope, TransportRead, TransportWrite, Unpack,
    UnpackResult,
};

/// # Intermediate
///
/// In case 4-byte data alignment is needed,
/// an intermediate version of the original protocol may be used.
///
/// * Overhead: small
/// * Minimum envelope length: 4 bytes
/// * Maximum envelope length: 4 bytes
///
/// Payload structure:
///
/// ```text
/// +----+----...----+
/// +len.+  payload  +
/// +----+----...----+
/// ```
///
/// Before sending anything into the underlying socket (see [transports]),
/// the client must first send `0xeeeeeeee` as the first int (four bytes,
/// the server **will not** send `0xeeeeeeee` as the first int in the first reply).
/// Then, payloads are wrapped in the following envelope:
///
/// * Length: payload length encoded as 4 length bytes (little endian)
/// * Payload: the MTProto payload
///
/// [Quick ACK »] may be enabled for this transport.
///
/// To request a quick ACK from the server for an encrypted MTProto payload,
/// add `0x80000000` to the `len` field before encoding it (equivalent to doing
/// `len = len | (1 << 31)`, i.e. set the most-significant bit of the length).
///
/// The server will send quick ACK tokens as a
/// standalone 4-byte packet without a length header.
///
/// ```text
/// +----+
/// |abcd|
/// +----+
/// ```
///
/// These quick ACK packets can be easily distinguished from normal
/// intermediate packets because quick ACK tokens always have the
/// most-significant bit of the last byte set, and trying to decode an ACK
/// token as a little-endian 32-bit integer will always yield a value bigger
/// than or equal to `0x80000000`, which can never be a valid packet length.
///
/// ---
///
/// <https://core.telegram.org/mtproto/mtproto-transports#intermediate>
///
/// [transports]: https://core.telegram.org/mtproto/transports
/// [Quick ACK »]: https://core.telegram.org/mtproto/mtproto-transports#quick-ack
#[derive(Default)]
pub struct Intermediate;

pub struct IntermediateRead {
    _private: (),
}

pub struct IntermediateWrite {
    _private: (),
}

pub struct IntermediateEnvelope {
    header: unbite::Raw<4>,
}

impl crate::Sealed for Intermediate {}

impl Transport for Intermediate {
    type Read = IntermediateRead;
    type Write = IntermediateWrite;

    const INIT_SIZE: usize = 4;

    fn init(self, writer_buffer: &mut unbite::DynBuf) -> (Self::Read, Self::Write) {
        writer_buffer.extend_from_array(&[0xee, 0xee, 0xee, 0xee]);

        (
            IntermediateRead { _private: () },
            IntermediateWrite { _private: () },
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

        let len = u32::from_le_bytes(buffer[0..4].try_into().unwrap());

        if len & const { 1 << 31 } != 0 {
            return UnpackResult::Unpacked {
                result: Ok(Unpack::QuickAck(QuickAck { token: len })),
                offset: 4,
            };
        }

        let len = len as usize;

        if buffer.len() < len + 4 {
            return UnpackResult::Continue { length: len + 4 };
        }

        UnpackResult::Unpacked {
            result: Ok(Unpack::Packet(Packet { data: 4..4 + len })),
            offset: len + 4,
        }
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

impl TransportEnvelope for IntermediateEnvelope {
    #[inline]
    fn header_swap<const N: usize>(&mut self, buffer: &mut unbite::Raw<N>) {
        self.header.swap(buffer)
    }
}
