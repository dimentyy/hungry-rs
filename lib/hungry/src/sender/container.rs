use std::num::NonZeroU32;

use crate::mtproto::{EncryptedEnvelope, Msg};
use crate::pack::MsgContainer;
use crate::tl;
use crate::transport::Transport;

pub(super) struct Container<T: Transport> {
    transport: T::Envelope,
    encrypted: EncryptedEnvelope,
    raw_inner: MsgContainer,
}

impl<T: Transport> Container<T> {
    pub(crate) fn new(mut buffer: unbite::DynBuf) -> Container<T> {
        let transport = T::envelope(&mut buffer);
        let encrypted = EncryptedEnvelope::new(&mut buffer);

        Self {
            transport,
            encrypted,
            raw_inner: MsgContainer::new(buffer),
        }
    }

    #[inline]
    pub(super) fn len(&self) -> usize {
        self.raw_inner.len()
    }

    #[inline]
    pub(super) fn is_empty(&self) -> bool {
        self.raw_inner.is_empty()
    }

    #[inline]
    pub(super) fn can_push(&self, len: usize) -> bool {
        self.raw_inner.can_push(len)
    }

    pub(super) fn push<X: tl::Function>(&mut self, msg: Msg, x: &tl::ConstructorId<X>) {
        self.raw_inner.push(msg, x);
    }

    pub(super) fn finalize(self) -> (T::Envelope, EncryptedEnvelope, unbite::DynBuf) {
        let buffer = self.raw_inner.finalize();

        (self.transport, self.encrypted, buffer)
    }
}

pub(super) enum ContainerResult {
    Header(unbite::Raw<8>),
    Length(NonZeroU32),
}
