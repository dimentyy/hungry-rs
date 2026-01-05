mod error;
mod full;
mod intermediate;

#[cfg(feature = "obfuscated-transport")]
mod obfuscated;

use std::ops::{ControlFlow, Range};

pub use error::TransportError;
pub use full::Full;
pub use intermediate::Intermediate;

#[cfg(feature = "obfuscated-transport")]
pub use obfuscated::Obfuscated;

#[derive(Debug, Eq, PartialEq)]
pub struct QuickAck {}

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

pub(self) use bail;

pub trait Transport {
    type Read: TransportRead<Transport = Self>;
    type Init: TransportInit<Transport = Self>;
    type Write: TransportWrite<Transport = Self>;

    fn split(self) -> (Self::Read, Self::Init, Self::Write);

    fn envelope(buffer: &mut unbite::DynBuf) -> <Self::Write as TransportWrite>::Envelope;
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

    type Envelope;

    fn pack(&mut self, buffer: &mut unbite::DynBuf, envelope: Self::Envelope);
}

#[cfg(feature = "obfuscated-transport")]
pub trait IdentifiableTransport: Transport {
    const TRANSPORT_IDENTIFIER: [u8; 4];
}
