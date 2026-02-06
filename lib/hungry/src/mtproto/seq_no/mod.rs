mod error;

use std::fmt;

use crate::tl;

use tl::Identifiable;

pub use error::SeqNoError;

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

#[inline]
#[must_use]
pub const fn is_content_related(seq_no: SeqNo) -> bool {
    seq_no & 1 == 1
}

#[must_use]
#[derive(Debug, Default)]
pub struct SeqNos {
    current: SeqNo,
}

impl fmt::Display for SeqNos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "seq nos [current={}]", self.current)
    }
}

impl SeqNos {
    #[inline]
    pub const fn new() -> Self {
        Self { current: 0 }
    }

    #[inline]
    #[must_use]
    pub const fn non_content_related(&self) -> SeqNo {
        self.current * 2
    }

    #[inline]
    #[must_use]
    pub const fn get_content_related(&mut self) -> SeqNo {
        self.current += 1;
        (self.current * 2) - 1
    }

    #[expect(
        clippy::equatable_if_let,
        reason = "`std::cmp::PartialEq` is not yet stable as a const trait"
    )]
    pub const fn check(&mut self, seq_no: SeqNo, typ: u32) -> Result<(), SeqNoError> {
        use SeqNoError::*;

        let content_related = is_content_related(seq_no);

        let expected = if content_related {
            if let Some(false) = must_be_content_related(typ) {
                return Err(Odd);
            }

            self.get_content_related()
        } else {
            if let Some(true) = must_be_content_related(typ) {
                return Err(Even);
            }

            self.non_content_related()
        };

        if seq_no != expected {
            return Err(Invalid);
        }

        Ok(())
    }
}
