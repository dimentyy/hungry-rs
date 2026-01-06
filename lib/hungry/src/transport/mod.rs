#![forbid(unsafe_code)]

mod error;
mod full;
mod intermediate;

#[cfg(feature = "obfuscated-transport")]
mod obfuscated;

use std::ops::Range;

pub use error::TransportError;
pub use full::Full;
pub use intermediate::Intermediate;

#[cfg(feature = "obfuscated-transport")]
pub use obfuscated::Obfuscated;

/// # Quick ack
///
/// Some of the TCP transports listed above support quick ACKs: quick ACKs
/// are a way for clients to get quick receipt acknowledgements for packets.
///
/// To request a quick ack for a specific outgoing payload, clients
/// must set the MSB of an appropriate field in the transport envelope
/// (as described in the documentation for each transport protocol above).
///
/// Also, clients must generate and store a quick ACK token,
/// associating it with the outgoing MTProto payload, by:
///
/// * Taking the first 32 bits of the SHA256 of the encrypted portion of
///   the payload prepended by 32 bytes from the authorization key (the same
///   hash generated when computing the [message key], except that instead
///   of taking the middle 128 bits, the first 32 bits are taken instead).
///
/// * Setting the MSB of the last byte to 1: in other words, treat the 32 bits
///   generated above as a little-endian integer, then add `0x80000000` to it
///   (i.e. `ack_token = msg_key_long[0:4] | (1 << 31)` on a little-endian system).
///
/// Once the payload is successfully received, decrypted and accepted
/// for processing by the server, the server will send back the
/// same quick ACK token we generated above, using the encoding
/// described in the documentation for each transport protocol.
///
/// Note that reception of a quick ACK **does not** indicate that any of
/// the RPC queries contained in the message have succeeded, failed
/// or finished execution at all, it simply indicates that they have
/// been received, decrypted and accepted for processing by the server.
///
/// The server will still send `msgs_ack` constructors for content-related
/// constructors and methods contained in payloads which were quick ACKed,
/// as well as replies/errors for methods and constructors, as usual.
///
/// ---
///
/// https://core.telegram.org/mtproto/mtproto-transports#quick-ack
///
/// [message key]: https://core.telegram.org/mtproto/description#message-key-msg-key
#[derive(Debug, Eq, PartialEq)]
pub struct QuickAck {
    pub token: u32
}

#[derive(Debug, Eq, PartialEq)]
pub struct Packet {
    pub data: Range<usize>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Unpack {
    Packet(Packet),
    QuickAck(QuickAck),
}

#[derive(Debug, Eq, PartialEq)]
pub enum UnpackResult {
    Unpacked {
        result: Result<Unpack, TransportError>,
        offset: usize,
    },
    Continue {
        length: usize,
    },
}

macro_rules! bail {
    (offset: $offset:expr => $variant:ident $( $tokens:tt )+ ) => {
        return UnpackResult::Unpacked { result: Err(TransportError::$variant $( $tokens )+ ), offset: $offset }
    };
}

use bail;

pub trait Transport: crate::Sealed {
    type Read: TransportRead<Transport = Self>;
    type Init: TransportInit<Transport = Self>;
    type Write: TransportWrite<Transport = Self>;

    fn split(self) -> (Self::Read, Self::Init, Self::Write);

    type Envelope;

    #[must_use]
    fn envelope(buffer: &mut unbite::DynBuf) -> Self::Envelope;
}

pub trait TransportRead {
    type Transport: Transport<Read = Self>;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult;
}

pub trait TransportInit {
    type Transport: Transport<Init = Self>;

    const SIZE: usize;

    fn init(self, write: &mut <Self::Transport as Transport>::Write, buffer: &mut unbite::DynBuf);
}

pub trait TransportWrite {
    type Transport: Transport<Write = Self>;

    fn pack(
        &mut self,
        buffer: &mut unbite::DynBuf,
        envelope: <Self::Transport as Transport>::Envelope,
    );
}

#[cfg(feature = "obfuscated-transport")]
pub trait IdentifiableTransport: Transport {
    const TRANSPORT_IDENTIFIER: [u8; 4];
}
