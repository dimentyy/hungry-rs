mod error;

use std::io;
use std::num::NonZeroUsize;
use std::pin::pin;
use std::task::{Context, Poll, ready};

use tokio::io::AsyncWrite;

use crate::transport::Transport;

pub use error::WriterError;

pub trait WriterDriver: AsyncWrite + Unpin {}
impl<T: AsyncWrite + Unpin> WriterDriver for T {}

pub struct Writer<W: WriterDriver, T: Transport> {
    driver: W,
    transport: T::Write,
}

impl<W: WriterDriver, T: Transport> Writer<W, T> {
    pub(crate) fn new(driver: W, transport: T::Write) -> Self {
        Self { driver, transport }
    }

    #[inline]
    pub fn driver(&mut self) -> &mut W {
        &mut self.driver
    }

    fn poll_checked(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<NonZeroUsize>> {
        let n = ready!(pin!(&mut self.driver).poll_write(cx, buf))?;

        assert!(
            n <= buf.len(),
            "`tokio::io::AsyncWrite` contract violation by `{}`: \
            reported number of bytes written ({n})\
             exceeds the buffer length ({})",
            std::any::type_name::<W>(),
            buf.len(),
        );

        let Some(n) = NonZeroUsize::new(n) else {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "wrote 0 bytes",
            )));
        };

        Poll::Ready(Ok(n))
    }
}
