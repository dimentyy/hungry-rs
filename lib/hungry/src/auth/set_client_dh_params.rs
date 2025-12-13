use std::fmt;

use rug::{Integer, integer::Order::MsfBe};

use crate::utils::SliceExt;
use crate::{crypto, mtproto, tl};

use tl::Int256;
use tl::mtproto::{funcs, types};

#[derive(Debug)]
pub enum DhGenOkError {
    NonceMismatch,
    ServerNonceMismatch,
    NewNonceHash1Mismatch,
}

impl fmt::Display for DhGenOkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DhGenOkError::*;

        f.write_str("`DhGenOk` validation error: ")?;

        f.write_str(match self {
            NonceMismatch => "`nonce` mismatch",
            ServerNonceMismatch => "`server_nonce` mismatch",
            NewNonceHash1Mismatch => "`new_nonce_hash1` mismatch",
        })
    }
}

impl std::error::Error for DhGenOkError {}

#[must_use]
pub struct SetClientDhParams {
    pub(crate) new_nonce: Int256,
    pub(crate) g: i32,
    pub(crate) dh_prime: Integer,
    pub(crate) g_a: Integer,
    pub(crate) server_time: i32,
    pub(crate) b: Integer,
    pub(crate) func: funcs::SetClientDhParams,
}

#[must_use]
fn new_nonce_hash(auth_key: &mtproto::AuthKey, new_nonce: &[u8; 32], number: u8) -> [u8; 16] {
    let mut data = [0; 32 + 1 + 8];

    data[..32].copy_from_slice(new_nonce);
    data[32] = number;
    data[33..].copy_from_slice(auth_key.aux_hash());

    *crypto::sha1!(data)[4..].arr()
}

impl SetClientDhParams {
    #[inline]
    pub fn func(&self) -> &funcs::SetClientDhParams {
        &self.func
    }

    pub fn dh_gen_ok(
        self,
        response: types::DhGenOk,
    ) -> Result<(mtproto::AuthKey, mtproto::Salt), DhGenOkError> {
        use DhGenOkError::*;

        if response.nonce != self.func.nonce {
            return Err(NonceMismatch);
        }

        if response.server_nonce != self.func.server_nonce {
            return Err(ServerNonceMismatch);
        }

        let mut data = [0; 256];

        let g_ab = self.g_a.pow_mod(&self.b, &self.dh_prime).unwrap();

        let len = g_ab.significant_digits::<u8>();

        g_ab.write_digits(&mut data[256 - len..], MsfBe);

        let auth_key = mtproto::AuthKey::new(data);

        if response.new_nonce_hash_1 != new_nonce_hash(&auth_key, &self.new_nonce, 1) {
            return Err(NewNonceHash1Mismatch);
        }

        let mut salt = i64::from_le_bytes(*self.new_nonce[..8].arr())
            ^ i64::from_le_bytes(*self.func.server_nonce[..8].arr());

        Ok((auth_key, salt))
    }
}
