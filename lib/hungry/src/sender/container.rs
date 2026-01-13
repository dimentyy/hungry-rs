use crate::mtproto::{EncryptedHeader, EncryptedPadding, Msg};
use crate::pack::MsgContainer;
use crate::tl;
use crate::transport::Transport;

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

    pub(super) fn push<X: tl::Function>(&mut self, msg: Msg, x: &tl::ConstructorId<X>) {
        self.container.push(msg, x);
    }

    pub(super) fn finalize(
        self,
    ) -> (
        T::Envelope,
        EncryptedHeader,
        EncryptedPadding,
        unbite::DynBuf,
    ) {
        (
            self.transport,
            self.header,
            self.pad,
            self.container.finalize(),
        )
    }
}
