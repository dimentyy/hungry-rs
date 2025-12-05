use crate::crypto;
use crate::tl::mtproto::{enums, funcs, types};
use crate::tl::ser::Serialize;
use crate::tl::{Int128, Int256};
use crate::utils::SliceExt;

#[inline]
pub fn start(nonce: Int128) -> ReqPqMulti {
    let func = funcs::ReqPqMulti { nonce };

    ReqPqMulti { func }
}

pub struct ReqPqMulti {
    func: funcs::ReqPqMulti,
}

impl ReqPqMulti {
    #[inline]
    pub fn func(&self) -> &funcs::ReqPqMulti {
        &self.func
    }

    pub fn res_pq(self, res_pq: enums::ResPq) -> ResPq {
        let enums::ResPq::ResPq(res_pq) = res_pq;

        if res_pq.nonce != self.func.nonce {
            todo!();
        }

        if res_pq.pq.len() != 8 {
            todo!();
        }

        let pq = u64::from_be_bytes(*res_pq.pq.arr());
        let (p, q) = crypto::factorize(pq);

        #[inline]
        fn without_leading_zeros(i: u64) -> Vec<u8> {
            let bytes = i.to_be_bytes();

            let index = bytes.iter().position(|&x| x != 0).unwrap_or(bytes.len());

            bytes[index..].to_vec()
        }

        ResPq {
            nonce: self.func.nonce,
            server_nonce: res_pq.server_nonce,
            server_public_key_fingerprints: res_pq.server_public_key_fingerprints,
            pq: res_pq.pq,
            p: without_leading_zeros(p),
            q: without_leading_zeros(q),
        }
    }
}

pub struct ResPq {
    nonce: Int128,
    server_nonce: Int128,
    server_public_key_fingerprints: Vec<i64>,
    pq: Vec<u8>,
    p: Vec<u8>,
    q: Vec<u8>,
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
        key: &'_ crypto::RsaKey,
    ) -> ReqDhParams<'_> {
        let public_key_fingerprint = key.fingerprint();

        if !self
            .server_public_key_fingerprints
            .contains(&public_key_fingerprint)
        {
            todo!()
        }

        let pq_inner_data = enums::PQInnerData::PQInnerData(types::PQInnerData {
            pq: self.pq.clone(),
            p: self.p.clone(),
            q: self.q.clone(),
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            new_nonce,
        });

        assert!(pq_inner_data.serialized_len() <= 144);

        unsafe { pq_inner_data.serialize_unchecked(random_padding_bytes.as_mut_ptr()) };

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

        ReqDhParams {
            data_with_padding,
            data_pad_reversed,
            key,
            func,
        }
    }
}

pub struct ReqDhParams<'a> {
    data_with_padding: [u8; 192],
    data_pad_reversed: [u8; 192],
    key: &'a crypto::RsaKey,
    func: funcs::ReqDhParams,
}

impl ReqDhParams<'_> {
    #[inline]
    pub fn key_aes_encrypted(
        &self,
        temp_key: &[u8; 32],
        key_aes_encrypted: &mut [u8; 256],
    ) -> bool {
        self.key.key_aes_encrypted(
            &self.data_with_padding,
            &self.data_pad_reversed,
            temp_key,
            key_aes_encrypted,
        )
    }

    pub fn func(&mut self, key_aes_encrypted: &[u8; 256]) -> &funcs::ReqDhParams {
        let encrypted_data = self.func.encrypted_data.arr_mut();

        let len = self.key.encrypted_data(key_aes_encrypted, encrypted_data);

        encrypted_data[..256 - len].fill(0);

        &self.func
    }
}
