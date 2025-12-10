mod error;
mod full;

use std::ops::{ControlFlow, Range, RangeFrom};

use bytes::BytesMut;

use crate::{Envelope, EnvelopeSize};

pub use error::Error;
pub use full::Full;

#[derive(Debug, Eq, PartialEq)]
pub struct QuickAck {
    pub token: u32,
    pub len: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Packet {
    pub data: Range<usize>,
    pub next: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Unpack {
    Packet(Packet),
    QuickAck(QuickAck),
}

pub trait Transport: EnvelopeSize {
    type Read: TransportRead<Transport = Self>;
    type Write: TransportWrite<Transport = Self>;

    #[must_use]
    fn split(self) -> (Self::Read, Self::Write);
}

pub trait TransportRead: Unpin {
    type Transport: Transport;

    const DEFAULT_BUF_LEN: usize;

    fn unpack(&mut self, buffer: &mut [u8]) -> ControlFlow<Result<Unpack, Error>, usize>;
}

pub trait TransportWrite: Unpin {
    type Transport: Transport;

    #[must_use]
    fn pack(
        &mut self,
        buffer: &mut BytesMut,
        envelope: Envelope<Self::Transport>,
    ) -> RangeFrom<usize>;
}
