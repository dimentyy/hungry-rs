use std::fmt;

#[must_use]
#[derive(Debug, Eq, PartialEq)]
pub enum TransportError {
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
            Status(code) => write!(f, "status code: {code}"),
            BadLen(len) => write!(f, "bad len: {len}"),
            BadCrc { received, computed } => write!(
                f,
                "bad crc: received {received:#010x}, computed {computed:#010x}"
            ),
            BadSeq { received, expected } => write!(
                f,
                "bad sequence number: received {received}, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for TransportError {}
