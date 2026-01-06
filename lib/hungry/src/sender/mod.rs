use crate::reader::{Reader, ReaderDriver};
use crate::transport::Transport;
use crate::writer::{QueuedWriter, WriterDriver};

pub struct Sender<T: Transport, R: ReaderDriver, W: WriterDriver> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,
}
