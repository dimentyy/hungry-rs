use rug::{integer, Integer};

use crate::utils::SliceExt;
use crate::{auth, crypto, tl};

use tl::mtproto::{enums, funcs};
use tl::Int256;

pub struct ReqDhParams<'a> {
    pub(crate) data_with_padding: [u8; 192],
    pub(crate) data_pad_reversed: [u8; 192],
    pub(crate) new_nonce: Int256,
    pub(crate) key: &'a crypto::RsaKey,
    pub(crate) func: funcs::ReqDhParams,
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

        let range = self.key.encrypted_data(key_aes_encrypted, encrypted_data);
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

    pub fn server_dh_params(self, server_dh_params: enums::ServerDhParams) -> auth::ServerDhParams {
        let mut server_dh_params = match server_dh_params {
            enums::ServerDhParams::ServerDhParamsFail(x) => todo!(),
            enums::ServerDhParams::ServerDhParamsOk(x) => x,
        };

        if server_dh_params.nonce != self.func.nonce {
            todo!()
        }

        if server_dh_params.server_nonce != self.func.server_nonce {
            todo!()
        }

        let encrypted_answer = &mut server_dh_params.encrypted_answer;

        if !encrypted_answer.len().is_multiple_of(16) {
            todo!()
        }

        if encrypted_answer.len() < 20 {
            todo!()
        }

        let (tmp_aes_key, tmp_aes_iv) =
            Self::compute_from_nonce(&self.func.server_nonce, &self.new_nonce);

        // * encrypted_answer := AES256_ige_encrypt (answer_with_hash, tmp_aes_key, tmp_aes_iv);
        // here, tmp_aes_key is a 256-bit key, and tmp_aes_iv is a 256-bit initialization vector.
        // The same as in all the other instances that use AES encryption, the encrypted data is
        // padded with random bytes to a length divisible by 16 immediately prior to encryption.
        crypto::aes_ige_decrypt(encrypted_answer, &tmp_aes_key, &mut tmp_aes_iv.clone());
        let answer_with_hash = server_dh_params.encrypted_answer;

        // * new_nonce_hash := 128 lower-order bits of SHA1 (new_nonce);
        // * answer := serialization server_DH_inner_data#b5890dba nonce:int128 server_nonce:int128
        // g:int dh_prime:string g_a:string server_time:int = Server_DH_inner_data;
        // * answer_with_hash := SHA1(answer) + answer + (0-15 random bytes);
        // such that the length be divisible by 16;
        let mut buf = tl::de::Buf::new(&answer_with_hash[20..]);

        let answer = match tl::de::Deserialize::deserialize_checked(&mut buf) {
            Ok(x) => x,
            Err(err) => panic!(),
        };

        let len = (answer_with_hash.len() - 20 - buf.len());
        let answer_sha1 = crypto::sha1!(&answer_with_hash[20..20 + len]);

        if &answer_with_hash[..20] != answer_sha1.as_slice() {
            todo!()
        }

        let enums::ServerDhInnerData::ServerDhInnerData(answer) = answer;

        if answer.nonce != self.func.nonce {
            todo!()
        }

        if answer.server_nonce != self.func.server_nonce {
            todo!()
        }

        let dh_prime = Integer::from_digits(&answer.dh_prime, integer::Order::MsfBe);
        let g_a = Integer::from_digits(&answer.g_a, integer::Order::MsfBe);

        auth::ServerDhParams {
            nonce: self.func.nonce,
            server_nonce: self.func.server_nonce,
            new_nonce: self.new_nonce,
            tmp_aes_key,
            tmp_aes_iv,
            g: answer.g,
            dh_prime,
            g_a,
            server_time: answer.server_time,
        }
    }
}
