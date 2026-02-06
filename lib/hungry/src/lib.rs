#![deny(
    unused_imports,
    clippy::undocumented_unsafe_blocks,
    clippy::missing_panics_doc,
    clippy::missing_safety_doc,
    clippy::perf,
    clippy::pedantic,
    clippy::nursery
)]
#![allow(
    clippy::enum_glob_use,
    clippy::doc_markdown,
    clippy::unreadable_literal,
    clippy::inline_always,
    clippy::missing_errors_doc
)]

mod private;

pub mod auth;
pub mod crypto;
pub mod mtproto;
pub mod pack;
pub mod plain;
pub mod reader;
pub mod sender;
pub mod transport;
pub mod unpack;
pub mod writer;

pub(crate) use private::Sealed;

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
    mut writer_buffer: unbite::DynBuf,
) -> (
    reader::Reader<R, T>,
    writer::OwnedWrite<W, T, unbite::DynBuf>,
) {
    let (reader_transport, writer_transport) = transport.init(&mut writer_buffer);

    let reader = reader::Reader::new(reader, reader_transport, reader_buffer);

    let writer = writer::Writer::new(writer, writer_transport);
    let owned_write = writer::OwnedWrite::new(writer, writer_buffer);

    (reader, owned_write)
}
