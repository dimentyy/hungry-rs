use std::fmt;

use crypto_bigint::{Encoding, Odd, U2048};
use digest::Digest;

use crate::{auth, common, crypto, tl};

use common::infallible;

use tl::Int256;
use tl::mtproto::{enums, funcs, types};

#[derive(Debug, Eq, PartialEq)]
pub enum ServerDhParamsOkError {
    NonceMismatch,
    ServerNonceMismatch,
    InvalidEncryptedAnswerLength,
    AnswerHashMismatch,
    InnerDeserialization(tl::de::Error),
    InnerNonceMismatch,
    InnerServerNonceMismatch,
    InvalidDhPrimeLen,
    EvenDhPrime,
    InvalidGALen,
}

impl fmt::Display for ServerDhParamsOkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ServerDhParamsOkError::*;

        f.write_str("`ServerDhParamsOk` validation error: ")?;

        f.write_str(match self {
            NonceMismatch => "`nonce` mismatch",
            ServerNonceMismatch => "`server_nonce` mismatch",
            InvalidEncryptedAnswerLength => "invalid `encrypted_answer` length",
            AnswerHashMismatch => "`answer` hash mismatch",
            InnerDeserialization(err) => return err.fmt(f),
            InnerNonceMismatch => "inner `nonce` mismatch",
            InnerServerNonceMismatch => "inner `server_nonce` mismatch",
            InvalidDhPrimeLen => "invalid `dh_prime` length",
            EvenDhPrime => "`dh_prime` is even",
            InvalidGALen => "invalid `g_a` length",
        })
    }
}

impl std::error::Error for ServerDhParamsOkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use ServerDhParamsOkError::*;

        match self {
            InnerDeserialization(err) => Some(err),
            _ => None,
        }
    }
}

impl From<tl::de::Error> for ServerDhParamsOkError {
    #[inline]
    fn from(value: tl::de::Error) -> Self {
        Self::InnerDeserialization(value)
    }
}

#[must_use]
#[derive(PartialEq)]
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

    pub fn server_dh_params_ok(
        &self,
        response: &types::ServerDhParamsOk,
    ) -> Result<auth::ServerDhParamsOk, ServerDhParamsOkError> {
        use ServerDhParamsOkError::*;

        if response.nonce != self.func.nonce {
            return Err(NonceMismatch);
        }

        if response.server_nonce != self.func.server_nonce {
            return Err(ServerNonceMismatch);
        }

        let mut encrypted_answer = response.encrypted_answer.clone();

        if !encrypted_answer.len().is_multiple_of(16) {
            return Err(InvalidEncryptedAnswerLength);
        }

        // * tmp_aes_key = SHA1(new_nonce + server_nonce) +
        // substr(SHA1(server_nonce + new_nonce), 0, 12);
        let mut tmp_aes_key = [0; 32];

        sha1::Sha1::new_with_prefix(&self.new_nonce)
            .chain_update(&response.server_nonce)
            .finalize_into(infallible!((&mut tmp_aes_key[..20]).try_into().unwrap()));

        let server_new_sha1 = sha1::Sha1::new_with_prefix(&response.server_nonce)
            .chain_update(&self.new_nonce)
            .finalize();

        tmp_aes_key[20..].copy_from_slice(&server_new_sha1[..12]);

        // * tmp_aes_iv = substr(SHA1(server_nonce + new_nonce), 12, 8) +
        // SHA1(new_nonce + new_nonce) + substr(new_nonce, 0, 4);
        let mut tmp_aes_iv = [0; 32];

        tmp_aes_iv[..8].copy_from_slice(&server_new_sha1[12..]);

        sha1::Sha1::new_with_prefix(&self.new_nonce)
            .chain_update(&self.new_nonce)
            .finalize_into(infallible!((&mut tmp_aes_iv[8..28]).try_into().unwrap()));

        tmp_aes_iv[28..].copy_from_slice(&self.new_nonce[..4]);

        // * encrypted_answer := AES256_ige_encrypt (answer_with_hash, tmp_aes_key, tmp_aes_iv);
        // here, tmp_aes_key is a 256-bit key, and tmp_aes_iv is a 256-bit initialization vector.
        // The same as in all the other instances that use AES encryption, the encrypted data is
        // padded with random bytes to a length divisible by 16 immediately prior to encryption.
        crypto::aes_ige_decrypt(&mut encrypted_answer, &tmp_aes_key, &mut tmp_aes_iv.clone());
        let answer_with_hash = encrypted_answer;

        // * new_nonce_hash := 128 lower-order bits of SHA1 (new_nonce);
        // * answer := serialization server_DH_inner_data#b5890dba nonce:int128 server_nonce:int128
        // g:int dh_prime:string g_a:string server_time:int = Server_DH_inner_data;
        // * answer_with_hash := SHA1(answer) + answer + (0-15 random bytes);
        // such that the length be divisible by 16;
        let mut buf = tl::de::Buf::new(&answer_with_hash[20..]);

        let enums::ServerDhInnerData::ServerDhInnerData(answer) = buf.de()?;

        let len = answer_with_hash.len() - 20 - buf.len();
        let answer_sha1 = sha1::Sha1::digest(&answer_with_hash[20..20 + len]);

        if &answer_with_hash[..20] != answer_sha1.as_slice() {
            return Err(AnswerHashMismatch);
        }

        if answer.nonce != self.func.nonce {
            return Err(InnerNonceMismatch);
        }

        if answer.server_nonce != self.func.server_nonce {
            return Err(InnerServerNonceMismatch);
        }

        let Ok(dh_prime) = answer.dh_prime.as_ref().try_into() else {
            return Err(InvalidDhPrimeLen);
        };
        let Some(dh_prime) = Odd::new(U2048::from_be_bytes(dh_prime)).into_option() else {
            return Err(EvenDhPrime);
        };

        let Ok(g_a) = answer.g_a.as_ref().try_into() else {
            return Err(InvalidGALen);
        };
        let g_a = U2048::from_be_bytes(g_a);

        Ok(auth::ServerDhParamsOk {
            nonce: self.func.nonce.clone(),
            server_nonce: self.func.server_nonce.clone(),
            new_nonce: self.new_nonce.clone(),
            tmp_aes_key,
            tmp_aes_iv,
            g: answer.g,
            dh_prime,
            g_a,
            server_time: answer.server_time,
        })
    }
}
