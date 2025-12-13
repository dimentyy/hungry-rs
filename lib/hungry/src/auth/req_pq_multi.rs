use std::fmt;

use crate::utils::SliceExt;
use crate::{auth, crypto, tl};

use tl::mtproto::{funcs, types};
use tl::Int128;

#[derive(Debug)]
pub enum ResPqError {
    NonceMismatch(auth::error::NonceMismatch),
    InvalidPqLen(InvalidPqLen),
}

impl fmt::Display for ResPqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ResPqError::*;

        f.write_str("`ResPq` validation error: ")?;

        match self {
            NonceMismatch(err) => err.fmt(f),
            InvalidPqLen(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ResPqError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use ResPqError::*;

        Some(match self {
            NonceMismatch(err) => err,
            InvalidPqLen(err) => err,
        })
    }
}

#[derive(Debug)]
pub struct InvalidPqLen {
    pub received: usize,
}

impl fmt::Display for InvalidPqLen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid `pq` len: {}", self.received)
    }
}

impl std::error::Error for InvalidPqLen {}

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
    pub fn new(nonce: Int128) -> Self {
        let func = funcs::ReqPqMulti { nonce };

        Self { func }
    }

    #[inline]
    pub fn func(&self) -> &funcs::ReqPqMulti {
        &self.func
    }

    #[inline]
    pub fn nonce(&self) -> &Int128 {
        &self.func.nonce
    }

    pub fn res_pq(&self, response: &types::ResPq) -> Result<auth::ResPq, ResPqError> {
        if response.nonce != self.func.nonce {
            return Err(ResPqError::NonceMismatch(auth::error::NonceMismatch {
                expected: self.func.nonce,
                received: response.nonce,
            }));
        }

        if response.pq.len() != 8 {
            return Err(ResPqError::InvalidPqLen(InvalidPqLen {
                received: response.pq.len(),
            }));
        }

        let pq = u64::from_be_bytes(*response.pq.arr());
        let (p, q) = crypto::factorize(pq);

        fn without_leading_zeros(i: u64) -> Vec<u8> {
            let bytes = i.to_be_bytes();

            let index = bytes.iter().position(|&x| x != 0).unwrap_or(bytes.len());

            bytes[index..].to_vec()
        }

        Ok(auth::ResPq {
            nonce: self.func.nonce,
            server_nonce: response.server_nonce,
            server_public_key_fingerprints: response.server_public_key_fingerprints.clone(),
            pq: response.pq.clone(),
            p: without_leading_zeros(p),
            q: without_leading_zeros(q),
        })
    }
}
