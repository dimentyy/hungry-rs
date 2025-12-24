mod bytes_ext;

pub(crate) use bytes_ext::{BytesMutExt, unsplit_checked};

macro_rules! ready_ok {
    ($e:expr) => {{
        use std::task::Poll;

        match $e {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Ok(ok)) => ok,
            Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
        }
    }};
}

pub(crate) use ready_ok;
