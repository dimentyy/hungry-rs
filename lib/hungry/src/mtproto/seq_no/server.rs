use crate::mtproto::{
    ClientSeqNos, SeqNo, SeqNoError, is_content_related, must_be_content_related,
};

#[must_use]
pub struct ServerSeqNos {
    /// The client counterpart is used here to
    /// avoid repeated `seq_no` iteration code.
    client: ClientSeqNos,
    missed: Vec<SeqNo>,
}

impl ServerSeqNos {
    #[inline]
    pub const fn new() -> Self {
        Self {
            client: ClientSeqNos::new(),
            missed: Vec::new(),
        }
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

            self.client.get_content_related()
        } else {
            if let Some(true) = must_be_content_related(typ) {
                return Err(Even);
            }

            self.client.non_content_related()
        };

        if seq_no != expected {
            return Err(Invalid);
        }

        Ok(())
    }
}
