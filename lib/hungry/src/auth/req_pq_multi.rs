use std::fmt;

use crate::{auth, crypto, tl};

use tl::mtproto::{funcs, types};

#[must_use]
#[derive(Debug)]
pub enum ResPqError {
    NonceMismatch,
    InvalidPqLen,
    Factorization
}

impl fmt::Display for ResPqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ResPqError::*;

        f.write_str("`ResPq` validation error: ")?;

        f.write_str(match self {
            NonceMismatch => "`nonce` mismatch",
            InvalidPqLen => "invalid `pq` length",
            Factorization => "failed to factorize `pq`"
        })
    }
}

impl std::error::Error for ResPqError {}

#[must_use]
pub struct ReqPqMulti {
    func: funcs::ReqPqMulti,
}

impl fmt::Debug for ReqPqMulti {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReqPqMulti")
            .field("nonce", self.nonce())
            .finish()
    }
}

impl ReqPqMulti {
    #[inline]
    pub fn new(nonce: tl::Int128) -> Self {
        let func = funcs::ReqPqMulti { nonce };

        Self { func }
    }

    #[inline]
    #[must_use]
    pub fn func(&self) -> &funcs::ReqPqMulti {
        &self.func
    }

    #[inline]
    #[must_use]
    pub fn nonce(&self) -> &tl::Int128 {
        &self.func.nonce
    }

    pub fn res_pq(&self, response: &types::ResPq) -> Result<auth::ResPq, ResPqError> {
        if response.nonce != self.func.nonce {
            return Err(ResPqError::NonceMismatch);
        }

        if response.pq.len() != 8 {
            return Err(ResPqError::InvalidPqLen);
        }

        let pq = u64::from_be_bytes(response.pq.as_slice().try_into().unwrap());

        let mut map = num_prime::nt_funcs::factorize64(pq);

        if map.len() != 2 {
            return Err(ResPqError::Factorization)
        }

        let Some((p, 1)) = map.pop_first() else {
            return Err(ResPqError::Factorization)
        };

        let Some((q, 1)) = map.pop_last() else {
            return Err(ResPqError::Factorization)
        };

        Ok(auth::ResPq {
            nonce: self.func.nonce.clone(),
            server_nonce: response.server_nonce.clone(),
            server_public_key_fingerprints: response.server_public_key_fingerprints.clone(),
            pq: response.pq.clone(),
            p: tl::Bytes(crypto::trim_zeroes_left(&p.to_be_bytes()).to_vec()),
            q: tl::Bytes(crypto::trim_zeroes_left(&q.to_be_bytes()).to_vec()),
        })
    }
}
