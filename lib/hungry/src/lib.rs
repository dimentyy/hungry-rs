#![allow(unused)]

mod envelope;

pub mod auth;
pub mod crypto;
pub mod mtproto;
pub mod plain;
pub mod reader;
pub mod transport;
pub mod utils;
pub mod writer;

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

pub use rug;

pub use hungry_tl as tl;

pub(crate) use envelope::envelopes;

pub use envelope::{Envelope, EnvelopeSize};

pub fn new<
    T: transport::Transport + Default,
    R: AsyncRead + Unpin,
    H: reader::Handle,
    W: AsyncWrite + Unpin,
>(
    reader: R,
    reader_handle: H,
    reader_buffer: BytesMut,
    writer: W,
) -> (reader::Reader<R, H, T>, writer::Writer<W, T>) {
    let (reader_transport, writer_transport) = T::default().split();

    let writer = writer::Writer::new(writer, writer_transport);
    let reader = reader::Reader::new(reader, reader_handle, reader_transport, reader_buffer);

    (reader, writer)
}
