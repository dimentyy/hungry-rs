use std::pin::Pin;
use std::task::{Context, Poll, ready};

use tokio::io::AsyncWrite;

use crate::transport::Transport;
use crate::writer::{Writer, WriterError};

pub struct OwnedWriteInner<W: AsyncWrite + Unpin, T: Transport, B: AsRef<[u8]>> {
    pub driver: Writer<W, T>,
    pub buffer: B,
}

pub struct OwnedWrite<W: AsyncWrite + Unpin, T: Transport, B: AsRef<[u8]>> {
    inner: Option<OwnedWriteInner<W, T, B>>,
    pos: usize,
}

impl<W: AsyncWrite + Unpin, T: Transport, B: AsRef<[u8]>> OwnedWrite<W, T, B> {
    pub(crate) fn new(driver: Writer<W, T>, buffer: B) -> Self {
        Self {
            inner: Some(OwnedWriteInner { driver, buffer }),
            pos: 0,
        }
    }

    /// Consumes the [`OwnedWrite`] returning its inner data with the current position.
    ///
    /// # Panics
    ///
    /// * If the method was called after completion.
    pub fn into_inner(self) -> (OwnedWriteInner<W, T, B>, usize) {
        let inner = self.inner.expect("called `into_inner` after completion");

        (inner, self.pos)
    }

    /// # Panics
    ///
    /// * If the method was called after completion.
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

            let n = ready!(driver.poll_checked(cx, buf)).map_err(WriterError::Io)?;

            self.pos += n.get();
        }
    }
}

impl<W: AsyncWrite + Unpin, T: Transport, B: AsRef<[u8]> + Unpin> Future for OwnedWrite<W, T, B>
where
    T::Write: Unpin,
{
    type Output = Result<OwnedWriteInner<W, T, B>, WriterError>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.get_mut().poll(cx)
    }
}
