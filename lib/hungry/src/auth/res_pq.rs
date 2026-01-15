use crate::{auth, crypto, tl};

use tl::mtproto::{enums, funcs, types};

#[must_use]
#[derive(Debug)]
pub struct ResPq {
    pub(super) nonce: tl::Int128,
    pub(super) server_nonce: tl::Int128,
    pub(super) server_public_key_fingerprints: Vec<crypto::RsaKeyFingerprint>,
    pub(super) pq: tl::Bytes,
    pub(super) p: tl::Bytes,
    pub(super) q: tl::Bytes,
}

impl ResPq {
    #[inline]
    #[must_use]
    pub fn server_public_key_fingerprints(&self) -> &[crypto::RsaKeyFingerprint] {
        self.server_public_key_fingerprints.as_slice()
    }

    /// Panics
    ///
    /// * If the provided `server_public_key` fingerprint is not found.
    pub fn req_dh_params<'a>(
        &self,
        mut random_padding_bytes: [u8; 192],
        new_nonce: tl::Int256,
        server_public_key: &'a crypto::RsaKey,
    ) -> auth::ReqDhParams<'a> {
        let fingerprint = server_public_key.fingerprint();

        assert!(
            self.server_public_key_fingerprints.contains(&fingerprint),
            "invalid fingerprint of the provided `server_public_key`"
        );

        let pq_inner_data: enums::PQInnerData = types::PQInnerData {
            pq: self.pq.clone(),
            p: self.p.clone(),
            q: self.q.clone(),
            nonce: self.nonce.clone(),
            server_nonce: self.server_nonce.clone(),
            new_nonce: new_nonce.clone(),
        }
        .into();

        // FIXME: simplify.
        let mut buf = tl::ser::Buf::new(&mut random_padding_bytes);
        buf.ser(&pq_inner_data);

        let data_with_padding = random_padding_bytes;

        let encrypted_data = tl::Bytes(Vec::with_capacity(256));

        let func = funcs::ReqDhParams {
            nonce: self.nonce.clone(),
            server_nonce: self.server_nonce.clone(),
            p: self.p.clone(),
            q: self.q.clone(),
            public_key_fingerprint: fingerprint,
            encrypted_data,
        };

        auth::ReqDhParams {
            data_with_padding,
            new_nonce,
            server_public_key,
            func,
        }
    }
}
