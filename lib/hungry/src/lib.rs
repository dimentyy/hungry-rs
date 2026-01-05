#![forbid(clippy::undocumented_unsafe_blocks)]

mod reader;
mod writer;

pub mod auth;
pub mod crypto;
pub mod mtproto;
pub mod transport;

pub use crypto_bigint;

pub use unbite;

pub use hungry_tl as tl;

pub use tl::common;

pub fn init<
    T: transport::Transport,
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
>(
    transport: T,
    reader: R,
    reader_buffer: unbite::DynBuf,
    writer: W,
) -> (reader::Reader<R, T>, T::Init, writer::Writer<W, T>) {
    let (reader_transport, init, writer_transport) = transport.split();

    let writer = writer::Writer::new(writer, writer_transport);
    let reader = reader::Reader::new(reader, reader_transport, reader_buffer);

    (reader, init, writer)
}
