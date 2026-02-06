#![forbid(unsafe_code, clippy::todo)]

mod error;
mod full;
mod intermediate;
mod quick_ack;

#[cfg(feature = "obfuscated-transport")]
mod obfuscated;

use std::ops::Range;

pub use error::TransportError;
pub use full::Full;
pub use intermediate::Intermediate;
pub use quick_ack::QuickAck;

#[cfg(feature = "obfuscated-transport")]
pub use obfuscated::Obfuscated;

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub struct Packet {
    pub data: Range<usize>,
}

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum Unpack {
    Packet(Packet),
    QuickAck(QuickAck),
}

#[must_use]
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
    type Write: TransportWrite<Transport = Self>;

    const INIT_SIZE: usize;

    #[must_use]
    fn init(self, writer_buffer: &mut unbite::DynBuf) -> (Self::Read, Self::Write);

    type Envelope: TransportEnvelope;

    #[must_use]
    fn envelope(buffer: &mut unbite::DynBuf) -> Self::Envelope;
}

pub trait TransportRead {
    type Transport: Transport<Read = Self>;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult;
}

pub trait TransportWrite {
    type Transport: Transport<Write = Self>;

    fn pack(
        &mut self,
        buffer: &mut unbite::DynBuf,
        envelope: <Self::Transport as Transport>::Envelope,
    );
}

pub trait TransportEnvelope {
    fn header_swap<const N: usize>(&mut self, buffer: &mut unbite::Raw<N>);
}

pub trait IdentifiableTransport: Transport {
    /// The protocol identifier, if its length
    /// is less than 4, it must be padded using
    /// the protocol identifier itself, to make
    /// its length 4 (`0xef` => `0xefefefef`).
    const TRANSPORT_IDENTIFIER: [u8; 4];
}
