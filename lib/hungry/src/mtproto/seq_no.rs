// STATUS: stable.

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
/// https://core.telegram.org/mtproto/description#message-sequence-number-msg-seqno
pub type SeqNo = i32;

#[must_use]
#[derive(Debug, Default)]
pub struct SeqNos {
    current: i32,
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
}
