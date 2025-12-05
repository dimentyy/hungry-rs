mod bytes_ext;
mod dump;
mod slice_ext;

pub(crate) use bytes_ext::{unsplit_checked, BytesMutExt};
pub(crate) use slice_ext::SliceExt;

pub use dump::dump;

macro_rules! ready_ok {
    ($e:expr) => {{
        use std::task::{Poll, ready};

        match ready!($e) {
            Ok(ok) => ok,
            Err(err) => return Poll::Ready(Err(err.into())),
        }
    }};
}

pub(crate) use ready_ok;
