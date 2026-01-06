use std::task::{Context, Poll, ready};

use crate::transport::Transport;
use crate::writer::{Writer, WriterDriver, WriterError};

pub struct OwnedWriteInner<W: WriterDriver, T: Transport, B: AsRef<[u8]>> {
    pub driver: Writer<W, T>,
    pub buffer: B,
}

pub struct OwnedWrite<W: WriterDriver, T: Transport, B: AsRef<[u8]>> {
    inner: Option<OwnedWriteInner<W, T, B>>,
    pos: usize,
}

impl<W: WriterDriver, T: Transport, B: AsRef<[u8]>> OwnedWrite<W, T, B> {
    pub(crate) fn new(driver: Writer<W, T>, buffer: B) -> Self {
        Self {
            inner: Some(OwnedWriteInner { driver, buffer }),
            pos: 0
        }
    }
    
    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<OwnedWriteInner<W, T, B>, WriterError>> {
        let OwnedWriteInner { driver, buffer } =
            self.inner.as_mut().expect("called `poll` after completion");

        loop {
            let buf = &buffer.as_ref()[self.pos..];

            if buf.is_empty() {
                return Poll::Ready(Ok(self.inner.take().unwrap()));
            }

            let n = ready!(driver.poll_checked(cx, buf))?;

            self.pos += n.get();
        }
    }
}
