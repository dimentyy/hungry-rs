use std::marker::PhantomData;
use std::{mem, slice};

use bytes::BytesMut;

use crate::utils::{BytesMutExt, unsplit_checked};

/// A helper macro to define a standalone envelope size.
macro_rules! envelopes {
    { $( $vis:vis $alias:ident => $ident:ident : $header:expr , $footer:expr );+ $( ; )? } => { $(
        $vis struct $ident;

        impl crate::envelope::EnvelopeSize for $ident {
            const HEADER: usize = $header;
            const FOOTER: usize = $footer;
        }

        $vis type $alias = crate::envelope::Envelope<$ident>;
    )+ };
}

pub(crate) use envelopes;

/// A helper trait containing sizes for header and footer.
pub trait EnvelopeSize {
    const HEADER: usize;
    const FOOTER: usize;
}

/// Contains header and footer buffers of constant size.
pub struct Envelope<S: EnvelopeSize> {
    header: BytesMut,
    footer: BytesMut,
    _marker: PhantomData<S>,
}

impl<S: EnvelopeSize> Envelope<S> {
    /// Split an envelope from buffer. Buffer length is set to zero.
    #[must_use]
    pub fn split(buffer: &mut BytesMut) -> Self {
        assert!(
            buffer.capacity() >= S::HEADER + S::FOOTER,
            "buffer is not large enough to store envelope"
        );

        // SAFETY: the `buffer` is truncated afterward; envelope buffers must not be read.
        unsafe { buffer.set_full_len() };

        let header = buffer.split_to(S::HEADER);
        let footer = buffer.split_off(buffer.capacity() - S::FOOTER);

        buffer.clear();

        Self {
            header,
            footer,
            _marker: PhantomData,
        }
    }

    /// Get envelope buffers. May be uninitialized.
    #[must_use]
    #[inline(always)]
    pub(crate) fn buffers(&mut self) -> (&mut [u8], &mut [u8]) {
        (self.header.as_mut(), self.footer.as_mut())
    }

    /// Get a mutable contiguous slice of all buffers.
    #[must_use]
    pub(crate) fn unsplit_slice_mut(&mut self, buffer: &mut BytesMut) -> &mut [u8] {
        assert!(
            self.header.can_unsplit(buffer) && buffer.can_unsplit(&self.footer),
            "buffer does not belong to the envelope"
        );

        let len = S::HEADER + buffer.len() + S::FOOTER;

        // SAFETY: `buffer` belongs to the envelope.
        unsafe { slice::from_raw_parts_mut(self.header.as_mut_ptr(), len) }
    }

    /// Shrink buffer capacity to match its length. Excess buffer with remaining space is returned.
    #[must_use]
    pub fn adapt(&mut self, buffer: &mut BytesMut) -> Option<BytesMut> {
        // Do not check the header to allow the outer envelope to adapt to an inner envelope buffer.
        assert!(
            buffer.can_unsplit(&self.footer),
            "buffer does not belong to the envelope"
        );

        if !buffer.has_spare_capacity() {
            return None;
        }

        let len = buffer.len();

        // SAFETY: the `buffer` will be split to its length;
        // the `excess` buffer is truncated afterward;
        // envelope footer buffer must not be read.
        unsafe { buffer.set_full_len() };
        let footer = mem::take(&mut self.footer);
        buffer.unsplit(footer);

        self.footer = buffer.split_off(len);

        let mut excess = self.footer.split_off(S::FOOTER);
        excess.clear();
        Some(excess)
    }

    /// Join all the buffers back. Envelope length is added. Excess buffer length is not added.
    pub(crate) fn unsplit(self, buffer: &mut BytesMut, excess: Option<BytesMut>) {
        assert!(
            self.header.can_unsplit(buffer) && buffer.can_unsplit(&self.footer),
            "buffer does not belong to the envelope"
        );

        assert!(!buffer.has_spare_capacity(), "buffer is not full");

        buffer.unsplit_reverse(self.header);
        buffer.unsplit(self.footer);

        if let Some(mut excess) = excess {
            excess.clear();

            unsplit_checked!(
                buffer,
                excess,
                "excess buffer does not belong to the envelope"
            );
        }
    }
}
