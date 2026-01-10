#![forbid(unsafe_code)]

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

pub trait Transport: crate::Sealed + Unpin {
    type Read: TransportRead<Transport = Self>;
    type Write: TransportWrite<Transport = Self>;

    const INIT_SIZE: usize;

    fn init(self, writer_buffer: &mut unbite::DynBuf) -> (Self::Read, Self::Write);

    type Envelope;

    #[must_use]
    fn envelope(buffer: &mut unbite::DynBuf) -> Self::Envelope;
}

pub trait TransportRead: Unpin {
    type Transport: Transport<Read = Self>;

    fn unpack(&mut self, buffer: &mut [u8]) -> UnpackResult;
}

pub trait TransportWrite: Unpin {
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
