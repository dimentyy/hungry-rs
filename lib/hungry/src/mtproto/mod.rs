mod auth_key;
mod msg;
mod msg_id;
mod pack;
mod seq_no;
mod unpack;

use crate::envelopes;

pub use auth_key::{AuthKey, MsgKey};
pub use msg::{Msg, MsgContainer};
pub use msg_id::MsgIds;
pub use pack::{pack_encrypted, pack_plain};
pub use seq_no::SeqNos;
pub use unpack::{DecryptedMessage, EncryptedMessage, Message, PlainMessage};

pub const DECRYPTED_MESSAGE_HEADER_SIZE: usize = DecryptedMessage::HEADER_LEN
    + 8  // message_id
    + 4  // seq_no
    + 4; // message_data_length

/// For MTProto 2.0, the algorithm for computing
/// aes_key and aes_iv from auth_key and msg_key is
/// <...> where x = 0 for messages from client to
/// server and x = 8 for those from server to client.
///
/// https://core.telegram.org/mtproto/description#defining-aes-key-and-initialization-vector
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
        0;          // no padding
    pub Envelope => EnvelopeSize:
        EncryptedMessage::HEADER_LEN + DECRYPTED_MESSAGE_HEADER_SIZE,
        1024;       // padding (12..1024)
}
