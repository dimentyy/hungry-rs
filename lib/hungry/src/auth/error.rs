use std::fmt;

use crate::tl;

use tl::Int128;

#[derive(Debug)]
pub struct NonceMismatch {
    pub expected: Int128,
    pub received: Int128,
}

impl fmt::Display for NonceMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`nonce` mismatch: expected {:?}, received {:?}",
            self.expected, self.received
        )
    }
}

impl std::error::Error for NonceMismatch {}
