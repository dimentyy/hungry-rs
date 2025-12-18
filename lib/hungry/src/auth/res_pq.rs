use crate::{auth, crypto, tl};

use tl::mtproto::{funcs, types};
use tl::ser::SerializeInto;
use tl::{Int128, Int256};

#[must_use]
pub struct ResPq {
    pub(super) nonce: Int128,
    pub(super) server_nonce: Int128,
    pub(super) server_public_key_fingerprints: Vec<i64>,
    pub(super) pq: Vec<u8>,
    pub(super) p: Vec<u8>,
    pub(super) q: Vec<u8>,
}

impl ResPq {
    #[inline]
    pub fn server_public_key_fingerprints(&self) -> &Vec<i64> {
        &self.server_public_key_fingerprints
    }

    pub fn req_dh_params(
        self,
        mut random_padding_bytes: [u8; 192],
        new_nonce: Int256,
        public_key: &'_ crypto::RsaKey,
    ) -> auth::ReqDhParams<'_> {
        let public_key_fingerprint = public_key.fingerprint();

        if !self
            .server_public_key_fingerprints
            .contains(&public_key_fingerprint)
        {
            panic!("invalid fingerprint of the provided `key`")
        }

        let pq_inner_data = tl::boxed(types::PQInnerData {
            pq: self.pq.clone(),
            p: self.p.clone(),
            q: self.q.clone(),
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            new_nonce,
        });

        random_padding_bytes.ser(&pq_inner_data);

        let data_with_padding = random_padding_bytes;

        let mut data_pad_reversed = data_with_padding;
        data_pad_reversed.reverse();

        let mut encrypted_data = Vec::with_capacity(256);
        unsafe { encrypted_data.set_len(256) };

        let func = funcs::ReqDhParams {
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            p: self.p,
            q: self.q,
            public_key_fingerprint,
            encrypted_data,
        };

        auth::ReqDhParams {
            data_with_padding,
            data_pad_reversed,
            new_nonce,
            key: public_key,
            func,
        }
    }
}
