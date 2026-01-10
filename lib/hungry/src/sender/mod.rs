use tokio::io::{AsyncRead, AsyncWrite};

use crate::reader::Reader;
use crate::transport::Transport;
use crate::writer::QueuedWriter;

pub struct Sender<T: Transport, R: AsyncRead + Unpin, W: AsyncWrite + Unpin> {
    reader: Reader<R, T>,
    writer: QueuedWriter<W, T>,
}
