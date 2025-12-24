use std::fmt;

use rug::{Integer, integer::Order::MsfBe};

use crate::{auth, crypto, tl};

use tl::Int256;
use tl::mtproto::{enums, funcs, types};

#[derive(Debug)]
pub enum ServerDhParamsOkError {
    NonceMismatch,
    ServerNonceMismatch,
    InvalidEncryptedAnswerLength,
    AnswerHashMismatch,
    InnerDeserialization(tl::de::Error),
    InnerNonceMismatch,
    InnerServerNonceMismatch,
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
            InnerNonceMismatch => "`answer` `nonce` mismatch",
            InnerServerNonceMismatch => "`answer` `server_nonce` mismatch",
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
    fn from(value: tl::de::Error) -> Self {
        Self::InnerDeserialization(value)
    }
}

#[must_use]
pub struct ReqDhParams<'a> {
    pub(crate) data_with_padding: [u8; 192],
    pub(crate) data_pad_reversed: [u8; 192],
    pub(crate) new_nonce: Int256,
    pub(crate) key: &'a crypto::RsaKey,
    pub(crate) func: funcs::ReqDhParams,
}

/// Wrapper around `key_aes_encrypted: &[u8; 256]` to prevent
/// arbitrary data for being passed into [`ReqDhParams::func`].
#[must_use]
pub struct KeyAesEncrypted<'a>(&'a [u8; 256]);

impl ReqDhParams<'_> {
    #[inline]
    pub fn key_aes_encrypted<'a>(
        &self,
        temp_key: &[u8; 32],
        key_aes_encrypted: &'a mut [u8; 256],
    ) -> Option<KeyAesEncrypted<'a>> {
        let success = self.key.key_aes_encrypted(
            &self.data_with_padding,
            &self.data_pad_reversed,
            temp_key,
            key_aes_encrypted,
        );

        if success {
            Some(KeyAesEncrypted(key_aes_encrypted))
        } else {
            None
        }
    }

    pub fn func(&mut self, key_aes_encrypted: KeyAesEncrypted) -> &funcs::ReqDhParams {
        let mut encrypted_data: &mut [u8; 256] =
            self.func.encrypted_data.as_mut_slice().try_into().unwrap();

        let range = self.key.encrypted_data(key_aes_encrypted.0, encrypted_data);
        encrypted_data[range].fill(0);

        &self.func
    }

    fn compute_from_nonce(
        server_nonce: &[u8; 16],
        new_nonce: &[u8; 32],
    ) -> (crypto::AesIgeKey, crypto::AesIgeIv) {
        let new_server_sha1 = crypto::sha1!(new_nonce, server_nonce);
        let server_new_sha1 = crypto::sha1!(server_nonce, new_nonce);
        let new_new_sha1 = crypto::sha1!(new_nonce, new_nonce);

        // * tmp_aes_key = SHA1(new_nonce + server_nonce) +
        // substr(SHA1(server_nonce + new_nonce), 0, 12);
        let mut aes_key = [0; 32];
        aes_key[..20].copy_from_slice(&new_server_sha1);
        aes_key[20..].copy_from_slice(&server_new_sha1[..12]);

        // * tmp_aes_iv = substr(SHA1(server_nonce + new_nonce), 12, 8) +
        // SHA1(new_nonce + new_nonce) + substr(new_nonce, 0, 4);
        let mut aes_iv = [0; 32];
        aes_iv[..8].copy_from_slice(&server_new_sha1[12..]);
        aes_iv[8..28].copy_from_slice(&new_new_sha1);
        aes_iv[28..].copy_from_slice(&new_nonce[..4]);

        (aes_key, aes_iv)
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

        let (tmp_aes_key, tmp_aes_iv) =
            Self::compute_from_nonce(&self.func.server_nonce, &self.new_nonce);

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

        let answer = buf.de()?;

        let len = (answer_with_hash.len() - 20 - buf.len());
        let answer_sha1 = crypto::sha1!(&answer_with_hash[20..20 + len]);

        if &answer_with_hash[..20] != answer_sha1.as_slice() {
            return Err(AnswerHashMismatch);
        }

        let enums::ServerDhInnerData::ServerDhInnerData(answer) = answer;

        if answer.nonce != self.func.nonce {
            return Err(InnerNonceMismatch);
        }

        if answer.server_nonce != self.func.server_nonce {
            return Err(InnerServerNonceMismatch);
        }

        let dh_prime = Integer::from_digits(&answer.dh_prime, MsfBe);
        let g_a = Integer::from_digits(&answer.g_a, MsfBe);

        Ok(auth::ServerDhParamsOk {
            nonce: self.func.nonce,
            server_nonce: self.func.server_nonce,
            new_nonce: self.new_nonce,
            tmp_aes_key,
            tmp_aes_iv,
            g: answer.g,
            dh_prime,
            g_a,
            server_time: answer.server_time,
        })
    }
}
