use std::fmt;

use crate::{auth, crypto, tl};

use tl::Int256;
use tl::mtproto::{enums, funcs, types};

#[must_use]
pub struct ReqDhParams<'a> {
    pub(crate) data_with_padding: [u8; 192],
    pub(crate) new_nonce: Int256,
    pub(crate) server_public_key: &'a crypto::RsaKey,
    pub(crate) func: funcs::ReqDhParams,
}

impl ReqDhParams<'_> {
    pub fn func(&mut self, temp_key: &[u8; 32]) -> Option<&funcs::ReqDhParams> {
        let encrypted_data = self
            .server_public_key
            .encrypted_data(&self.data_with_padding, temp_key)?;

        self.func.encrypted_data.clear();
        self.func
            .encrypted_data
            .extend_from_slice(&encrypted_data.to_be_bytes());

        Some(&self.func)
    }
}
