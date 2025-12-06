mod auth_key;
mod pack;
mod unpack;

use crate::envelopes;

pub use auth_key::AuthKey;
pub use pack::{pack_encrypted, pack_plain};
pub use unpack::{DecryptedMessage, EncryptedMessage, Message, PlainMessage};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Side {
    Client = 0,
    Server = 8,
}

impl Side {
    #[inline]
    pub const fn x(self) -> usize {
        self as usize
    }
}

envelopes! {
    pub PlainEnvelope => PlainEnvelopeSize:
        8 + 8 + 4,  // auth_key_id (8), message_id (8), message_data_length (4)
        0;
    pub Envelope => EnvelopeSize:
        8 + 16 + 8 + 8, // auth_key_id (8), msg_key (16), salt (8), session_id (8)
        1024;           // padding (12..1024)
}
