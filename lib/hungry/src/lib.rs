#![allow(clippy::uninit_vec)]
#![deny(unused_imports)]

mod envelope;
mod gzip_packed;
mod msg_container;

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
pub use msg_container::MsgContainer;

pub fn new<
    T: transport::Transport,
    R: AsyncRead + Unpin,
    H: reader::HandleReader,
    W: AsyncWrite + Unpin,
>(
    transport: T,
    reader: R,
    reader_handle: H,
    reader_buffer: BytesMut,
    writer: W,
) -> (reader::Reader<R, T, H>, writer::Writer<W, T>) {
    let (reader_transport, writer_transport) = transport.split();

    let writer = writer::Writer::new(writer, writer_transport);
    let reader = reader::Reader::new(reader, reader_transport, reader_handle, reader_buffer);

    (reader, writer)
}
