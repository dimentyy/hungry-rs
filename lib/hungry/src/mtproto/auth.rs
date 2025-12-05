use crate::crypto;
use crate::tl::mtproto::{enums, funcs};
use crate::tl::Int128;
use crate::utils::SliceExt;

#[inline]
pub fn start(nonce: Int128) -> ReqPq {
    let func = funcs::ReqPq { nonce };

    ReqPq { func }
}

#[derive(Debug)]
pub struct ReqPq {
    func: funcs::ReqPq,
}

impl ReqPq {
    #[inline]
    pub fn func(&self) -> &funcs::ReqPq {
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
        fn to_vec(i: u64) -> Vec<u8> {
            let bytes = i.to_be_bytes();

            let index = bytes.iter().position(|x| *x != 0).unwrap_or(0);

            bytes[index..].to_vec()
        }

        ResPq {
            nonce: self.func.nonce,
            server_nonce: res_pq.server_nonce,
            server_public_key_fingerprints: res_pq.server_public_key_fingerprints,
            pq: res_pq.pq,
            p: to_vec(p),
            q: to_vec(q),
        }
    }
}

#[derive(Debug)]
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
}
