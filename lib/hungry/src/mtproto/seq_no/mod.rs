mod client;
mod error;
mod server;

use crate::tl;

use tl::Identifiable;

pub use client::ClientSeqNos;
pub use error::SeqNoError;
pub use server::ServerSeqNos;

/// # Message Sequence Number (msg_seqno)
///
/// A 32-bit number equal to twice the number of [content-related »] messages
/// created by the sender prior to this message and subsequently incremented
/// by one if the current message is a content-related message.
///
/// The seqno of a content-related message is thus `msg.seqNo = (current_seqno*2)+1`
/// (and after generating it, the local `current_seqno` counter must be incremented by 1),
/// the seqno of a non-content related message is `msg.seqNo = (current_seqno*2)`
/// (`current_seqno` must not be incremented by 1 after generation).
///
/// Thus, the content-relatedness of an incoming MTProto message can simply be
/// determined by checking the value of the least-significant bit of the seqno
/// of the message (`message.isContentRelated = (message.seqNo & 1) == 1`).
///
/// A container is always generated after its entire contents;
/// therefore, its sequence number is greater than or equal
/// to the sequence numbers of the messages contained in it.
///
/// [content-related »]: https://core.telegram.org/mtproto/description#content-related-message
///
/// ---
///
/// <https://core.telegram.org/mtproto/description#message-sequence-number-msg-seqno>
pub type SeqNo = i32;

// FIXME.
#[inline]
#[must_use]
pub const fn must_be_content_related(typ: u32) -> Option<bool> {
    match typ {
        tl::mtproto::types::MsgsAck::CONSTRUCTOR_ID | tl::MSG_CONTAINER | tl::GZIP_PACKED => {
            Some(false)
        }
        _ => None,
    }
}

#[must_use]
#[inline(always)]
pub const fn is_content_related(seq_no: SeqNo) -> bool {
    seq_no & 1 == 1
}
