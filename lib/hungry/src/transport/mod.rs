mod error;
mod full;

use std::ops::{Range, RangeFrom};

use bytes::BytesMut;

use crate::{Envelope, EnvelopeSize};

pub(self) use error::err;

pub use error::{Error, ErrorKind};
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

    fn split(self) -> (Self::Read, Self::Write);
}

pub trait TransportRead: Unpin {
    type Transport: Transport;

    fn length(&mut self, buffer: &mut [u8]) -> usize;

    fn unpack(&mut self, buffer: &mut [u8]) -> Result<Unpack, Error>;
}

pub trait TransportWrite: Unpin {
    type Transport: Transport;

    fn pack(
        &mut self,
        buffer: &mut BytesMut,
        envelope: Envelope<Self::Transport>,
    ) -> RangeFrom<usize>;
}
