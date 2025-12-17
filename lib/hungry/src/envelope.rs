use std::marker::PhantomData;
use std::mem;

use bytes::BytesMut;

use crate::utils::{BytesMutExt, unsplit_checked};

/// A helper macro to define a standalone envelope size.
macro_rules! envelopes {
    { $( $vis:vis $name:ident => $size:ident: $header:expr, $footer:expr );+ $(;)? } => { $(
        $vis struct $size;

        impl crate::envelope::EnvelopeSize for $size {
            const HEADER: usize = $header;
            const FOOTER: usize = $footer;
        }

        $vis type $name = crate::envelope::Envelope<$size>;
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
    #[inline]
    pub(crate) fn buffers(&mut self) -> (&mut [u8], &mut [u8]) {
        (self.header.as_mut(), self.footer.as_mut())
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
