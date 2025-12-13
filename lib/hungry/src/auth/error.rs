use std::fmt;

use crate::{auth, tl};

use tl::Int128;

pub use auth::req_dh_params::ServerDhParamsOkError;
pub use auth::req_pq_multi::ResPqError;
pub use auth::set_client_dh_params::DhGenOkError;

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
