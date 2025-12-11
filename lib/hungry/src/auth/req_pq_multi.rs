use crate::utils::SliceExt;
use crate::{auth, crypto, tl};

use tl::mtproto::{funcs, types};

pub struct ReqPqMulti {
    pub(super) func: funcs::ReqPqMulti,
}

impl ReqPqMulti {
    #[inline]
    pub fn func(&self) -> &funcs::ReqPqMulti {
        &self.func
    }

    pub fn res_pq(self, response: types::ResPq) -> auth::ResPq {
        if response.nonce != self.func.nonce {
            todo!()
        }

        if response.pq.len() != 8 {
            todo!()
        }

        let pq = u64::from_be_bytes(*response.pq.arr());
        let (p, q) = crypto::factorize(pq);

        fn without_leading_zeros(i: u64) -> Vec<u8> {
            let bytes = i.to_be_bytes();

            let index = bytes.iter().position(|&x| x != 0).unwrap_or(bytes.len());

            bytes[index..].to_vec()
        }

        auth::ResPq {
            nonce: self.func.nonce,
            server_nonce: response.server_nonce,
            server_public_key_fingerprints: response.server_public_key_fingerprints,
            pq: response.pq,
            p: without_leading_zeros(p),
            q: without_leading_zeros(q),
        }
    }
}
