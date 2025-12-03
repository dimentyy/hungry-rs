use std::fmt;

macro_rules! err {
    ($kind:ident $( $args:tt )*) => {
        return Err(Error {
            kind: ErrorKind::$kind $( $args )*,
        })
    };
}

pub(super) use err;

#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("transport error: ")?;

        self.kind.fmt(f)
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug)]
pub enum ErrorKind {
    QuickAck,
    Status(i32),
    BadLen(i32),
    BadCrc { received: u32, computed: u32 },
    BadSeq { received: i32, expected: i32 },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ErrorKind::*;

        match self {
            QuickAck => write!(f, "quick ack is not supported"),
            Status(code) => write!(f, "status code: {code}"),
            BadLen(len) => write!(f, "bad len: {len}"),
            BadCrc {
                received: r,
                computed: c,
            } => write!(f, "bad crc: received {r:#010x}, computed {c:#010x}"),
            BadSeq {
                received: r,
                expected: e,
            } => write!(f, "bad seq: received {r}, expected {e}"),
        }
    }
}
