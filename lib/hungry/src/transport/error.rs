use std::fmt;

#[derive(Debug)]
pub enum TransportError {
    QuickAck,
    Status(i32),
    BadLen(i32),
    BadCrc { received: u32, computed: u32 },
    BadSeq { received: i32, expected: i32 },
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TransportError::*;

        f.write_str("transport error: ")?;

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

impl std::error::Error for TransportError {}
