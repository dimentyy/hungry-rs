use crate::reader::{Reader, ReaderDriver, ReaderError};
use crate::transport::Transport;
use crate::writer::{Writer, WriterDriver, WriterError};
use crate::{mtproto, tl};

#[must_use]
#[derive(Debug)]
pub enum PlainError {
    Reader(ReaderError),
    Writer(WriterError),
    EncryptedMessage(mtproto::EncryptedMessage),
}

#[must_use]
pub struct Plain<T: Transport, R: ReaderDriver, W: WriterDriver> {
    reader: Reader<R, T>,
    writer: Writer<W, T>,
}

impl<T: Transport, R: ReaderDriver, W: WriterDriver> Plain<T, R, W> {
    #[inline]
    pub async fn new(reader: Reader<R, T>, writer: Writer<W, T>) -> Self {
        Self { reader, writer }
    }

    pub fn send<F: tl::Function>(&mut self, f: &F) {}
}
