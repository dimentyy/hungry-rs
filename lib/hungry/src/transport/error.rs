use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum TransportError {
    /// # Transport errors
    ///
    /// In the event of a transport error (missing auth key, transport
    /// flood, etc.), the server may send a packet (framed as an MTProto
    /// payload by the chosen MTProto transport) with a signed little-endian
    /// number of 4 bytes, whose **absolute value** contains
    /// the error code (the error itself is actually negative).
    ///
    /// For example, error Code 403 corresponds to situations where the
    /// corresponding HTTP error would have been returned by the HTTP protocol.
    ///
    /// Error 404 (auth key not found) is returned when the specified auth key
    /// ID cannot be found by the DC, during the initial MTProto handshake if
    /// any of the specified queries is incorrect, or during normal operation
    /// for example if some MTProto fields are incorrect (i.e. the MTProto packet
    /// length is bigger than the transport-specified packet length, and so on).
    ///
    /// Error 429 (transport flood) is returned when too many transport
    /// connections are established to the same IP in a too short lapse of
    /// time, or if any of the [container]/[service message limits] are reached.
    ///
    /// Error 444 (invalid DC) is returned while
    /// [creating an auth key], [connecting to an MTProxy]
    /// or in other contexts if an invalid DC ID is specified.
    ///
    /// When using the [HTTP]/[HTTPS] transports, transport errors are not
    /// transmitted as specified above, instead they are simply returned
    /// as normal HTTP status codes (and the HTTP payload must be ignored).
    ///
    /// ---
    ///
    /// <https://cork.telegram.org/mtproto/mtproto-transports#transport-errors>
    ///
    /// [container]: https://core.telegram.org/mtproto/service_messages#simple-container
    /// [service message limits]: https://core.telegram.org/mtproto/service_messages_about_messages#acknowledgment-of-receipt
    /// [creating an auth key]: https://core.telegram.org/mtproto/auth_key#presenting-proof-of-work-server-authentication
    /// [connecting to an MTProxy]: https://core.telegram.org/mtproto/mtproto-transports#transport-obfuscation
    /// [HTTP]: https://core.telegram.org/mtproto/transports#http
    /// [HTTPS]: https://core.telegram.org/mtproto/transports#https
    Status(i32),

    BadLen(i32),

    BadSeq {
        received: i32,
        expected: i32,
    },

    BadCrc {
        received: u32,
        computed: u32,
    },
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TransportError::*;

        f.write_str("transport error: ")?;

        match self {
            Status(code) => write!(f, "status code: {code}"),
            BadLen(len) => write!(f, "bad len: {len}"),
            BadSeq { received, expected } => write!(
                f,
                "bad sequence number: received {received}, expected {expected}"
            ),
            BadCrc { received, computed } => write!(
                f,
                "bad crc: received {received:#010x}, computed {computed:#010x}"
            ),
        }
    }
}

impl std::error::Error for TransportError {}
