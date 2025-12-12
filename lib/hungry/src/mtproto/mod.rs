mod auth_key;
mod msg_id;
mod pack;
mod seq_no;
mod unpack;

pub mod tl;

use crate::envelopes;

pub use auth_key::{AuthKey, MsgKey};
pub use msg_id::MsgIds;
pub use pack::{pack_encrypted, pack_plain};
pub use seq_no::SeqNos;
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
        PlainMessage::HEADER_LEN,
        0;      // no padding
    pub Envelope => EnvelopeSize:
        EncryptedMessage::HEADER_LEN + DecryptedMessage::HEADER_LEN,
        1024;   // padding (12..1024)
}
