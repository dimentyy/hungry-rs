use crate::transport::Transport;
use crate::writer::{Writer, WriterDriver, WriterError};
use std::task::{Context, Poll, ready};

pub struct OwnedWriteInner<W: WriterDriver, T: Transport> {
    pub driver: Writer<W, T>,
    pub buffer: unbite::DynBuf,
}

pub struct OwnedWrite<W: WriterDriver, T: Transport> {
    inner: Option<OwnedWriteInner<W, T>>,
    pos: usize,
}

impl<W: WriterDriver, T: Transport> OwnedWrite<W, T> {
    pub(crate) fn new(driver: Writer<W, T>, buffer: unbite::DynBuf) -> Self {
        Self {
            inner: Some(OwnedWriteInner { driver, buffer }),
            pos: 0
        }
    }
    
    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<OwnedWriteInner<W, T>, WriterError>> {
        let OwnedWriteInner { driver, buffer } =
            self.inner.as_mut().expect("called `poll` after completion");

        loop {
            let buf = &buffer.as_slice()[self.pos..];

            if buf.is_empty() {
                return Poll::Ready(Ok(self.inner.take().unwrap()));
            }

            let n = ready!(driver.poll_checked(cx, buf))?;

            self.pos += n.get();
        }
    }
}
