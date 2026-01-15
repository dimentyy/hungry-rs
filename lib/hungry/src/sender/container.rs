use std::num::NonZeroU32;

use crate::mtproto::{EncryptedHeader, EncryptedPadding, Msg};
use crate::pack::{MsgContainer, MsgContainerResult};
use crate::tl;
use crate::transport::{Transport, TransportEnvelope};

pub(super) struct Container<T: Transport> {
    transport: T::Envelope,
    header: EncryptedHeader,
    pad: EncryptedPadding,
    container: MsgContainer,
}

impl<T: Transport> Container<T> {
    pub(crate) fn new(mut buffer: unbite::DynBuf) -> Container<T> {
        let transport = T::envelope(&mut buffer);
        let header = buffer.split_raw_front();
        let pad = buffer.split_raw_back();

        Self {
            transport,
            header,
            pad,
            container: MsgContainer::new(buffer),
        }
    }

    #[inline]
    pub(super) fn len(&self) -> usize {
        self.container.len()
    }

    #[inline]
    pub(super) fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

    #[inline]
    pub(super) fn can_push(&self, len: usize) -> bool {
        self.container.can_push(len)
    }

    pub(super) fn push<X: tl::Function>(&mut self, msg: Msg, x: &tl::ConstructorId<X>) {
        self.container.push(msg, x);
    }

    /// # Panics
    ///
    /// * If no messages were pushed to the container.
    pub(super) fn finalize(
        mut self,
    ) -> (
        ContainerResult,
        T::Envelope,
        EncryptedHeader,
        EncryptedPadding,
        unbite::DynBuf,
    ) {
        use MsgContainerResult::*;

        let (result, buffer) = match self.container.finalize() {
            Msg { mut header, buffer } => {
                self.header.swap(&mut header);
                self.transport.header_swap(&mut header);

                (ContainerResult::Header(header), buffer)
            }
            MsgContainer { length, buffer } => {
                (ContainerResult::Length(length), buffer)
            }
        };

        (
            result,
            self.transport,
            self.header,
            self.pad,
            buffer,
        )
    }
}

pub(super) enum ContainerResult {
    Header(unbite::Raw<8>),
    Length(NonZeroU32)
}
