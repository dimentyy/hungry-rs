use std::fmt;

use crate::mtproto::SeqNo;

#[must_use]
#[derive(Debug)]
pub struct ClientSeqNos {
    current: SeqNo,
}

impl Default for ClientSeqNos {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ClientSeqNos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "client seq nos [current={}]", self.current)
    }
}

impl ClientSeqNos {
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
        let current = self.current;

        self.current += 1;

        current * 2 + 1
    }
}
