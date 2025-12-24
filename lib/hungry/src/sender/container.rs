use bytes::BytesMut;

use crate::mtproto::{EncryptedEnvelope, Msg};
use crate::pack::MsgContainer;
use crate::transport::Transport;
use crate::{Envelope, tl};

pub(super) struct Container<T: Transport> {
    transport: Envelope<T>,
    encrypted: EncryptedEnvelope,
    container: MsgContainer,
}

impl<T: Transport> Container<T> {
    pub(crate) fn new(mut buffer: BytesMut) -> Container<T> {
        let transport = Envelope::split(&mut buffer);
        let encrypted = Envelope::split(&mut buffer);

        Self {
            container: MsgContainer::new(buffer),
            transport,
            encrypted,
        }
    }

    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        self.container.len()
    }

    #[inline(always)]
    pub(super) fn is_empty(&self) -> bool {
        self.container.is_empty()
    }

    #[inline(always)]
    pub(super) fn can_push(&self, len: usize) -> bool {
        self.container.can_push(len)
    }

    pub(super) fn push<X: tl::Function>(
        &mut self,
        msg: Msg,
        x: tl::CalculatedLen<'_, tl::ConstructorId<X>>,
    ) {
        self.container.push(msg, x);
    }

    pub(super) fn finalize(self) -> (Envelope<T>, EncryptedEnvelope, BytesMut) {
        (self.transport, self.encrypted, self.container.finalize())
    }
}
