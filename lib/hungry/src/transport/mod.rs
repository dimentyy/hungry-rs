mod error;
mod full;
mod intermediate;
mod obfuscated;

use std::ops::{ControlFlow, Range};

pub use error::TransportError;

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

pub trait Transport {
    type Read: TransportRead;
    type Write: TransportWrite;

    fn split(self) -> (Self::Read, Self::Write);
}

pub trait TransportRead {
    fn unpack(&mut self, buffer: &mut [u8]) -> ControlFlow<Result<Unpack, TransportError>, usize>;
}

pub trait TransportWrite {
    type Envelope: TransportEnvelope;

    fn pack(&mut self, buffer: &mut unbite::DynBuf, envelope: Self::Envelope);
}

pub trait TransportEnvelope {
    #[must_use]
    fn open(buffer: &mut unbite::DynBuf) -> Self;
}
