use crate::pack::MsgContainer;
use crate::transport::Transport;
use crate::{mtproto, tl};

pub(super) struct Container<T: Transport> {
    transport: T::Envelope,
    encrypted: mtproto::EncryptedEnvelope,
    raw_inner: MsgContainer,
}

impl<T: Transport> Container<T> {
    pub(crate) fn new(mut buffer: unbite::DynBuf) -> Self {
        let transport = T::envelope(&mut buffer);
        let encrypted = mtproto::EncryptedEnvelope::new(&mut buffer);

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

    #[inline]
    pub(super) fn push<F: FnOnce(&mut tl::ser::Buf)>(&mut self, msg: &mtproto::BytesMsg, f: F) {
        self.raw_inner.push(msg, f);
    }

    pub(super) fn finalize(self) -> (T::Envelope, mtproto::EncryptedEnvelope, unbite::DynBuf) {
        let buffer = self.raw_inner.finalize();

        (self.transport, self.encrypted, buffer)
    }
}
