use crypto_bigint::modular::{MontyForm, MontyParams};
use crypto_bigint::{Odd, One, U2048};
use digest::Digest;

use crate::{auth, crypto, tl};

use tl::SerializedLen;
use tl::mtproto::{enums, funcs, types};

#[must_use]
pub struct ServerDhParamsOk {
    pub(crate) nonce: tl::Int128,
    pub(crate) server_nonce: tl::Int128,
    pub(crate) new_nonce: tl::Int256,
    pub(crate) tmp_aes_key: crypto::AesIgeKey,
    pub(crate) tmp_aes_iv: crypto::AesIgeIv,
    pub(crate) g: i32,
    pub(crate) dh_prime: Odd<U2048>,
    pub(crate) g_a: U2048,
    pub(crate) server_time: i32,
}

impl ServerDhParamsOk {
    #[inline]
    #[must_use]
    pub fn server_time(&self) -> i32 {
        self.server_time
    }

    pub fn set_client_dh_params(&self, b: U2048, retry_id: i64) -> auth::SetClientDhParams {
        let _one = U2048::one();

        let g = U2048::from_u32(self.g as u32);

        // * g_b := pow(g, b) mod dh_prime
        let g_b = MontyForm::new(&g, MontyParams::new(self.dh_prime))
            .pow(&b)
            .retrieve();

        // TODO: checks

        // * data := serialization client_DH_inner_data#6643b654 nonce:int128
        // server_nonce:int128 retry_id:long g_b:string = Client_DH_Inner_Data
        let client_dh_inner_data: enums::ClientDhInnerData = types::ClientDhInnerData {
            nonce: self.nonce.clone(),
            server_nonce: self.server_nonce.clone(),
            retry_id,
            g_b: tl::Bytes(crypto::trim_zeroes_left(&g_b.to_be_bytes()).to_vec()),
        }
        .into();

        let serialized_len = client_dh_inner_data.serialized_len();

        // * data_with_hash := SHA1(data) + data + (0-15 random bytes);
        // such that length be divisible by 16;
        let mut data_with_hash = Vec::with_capacity((20 + serialized_len + 15) & !15);

        let data = tl::ser_uninit(
            &mut data_with_hash.spare_capacity_mut()[20..],
            &client_dh_inner_data,
        );

        let data_sha1 = sha1::Sha1::digest(data);
        data_with_hash.extend_from_slice(&data_sha1);

        // SAFETY: data is initialized.
        unsafe { data_with_hash.set_len(20 + serialized_len) };

        // TODO: allow custom random padding.
        getrandom::fill_uninit(data_with_hash.spare_capacity_mut()).unwrap();

        // SAFETY: spare capacity was filled with random bytes.
        unsafe { data_with_hash.set_len(data_with_hash.capacity()) };

        // * encrypted_data := AES256_ige_encrypt(data_with_hash, tmp_aes_key, tmp_aes_iv);
        crypto::aes_ige_encrypt(
            &mut data_with_hash,
            &self.tmp_aes_key,
            &mut self.tmp_aes_iv.clone(),
        );
        let encrypted_data = data_with_hash;

        let func = funcs::SetClientDhParams {
            nonce: self.nonce.clone(),
            server_nonce: self.server_nonce.clone(),
            encrypted_data: tl::Bytes(encrypted_data),
        };

        auth::SetClientDhParams {
            new_nonce: self.new_nonce.clone(),
            g: self.g,
            dh_prime: self.dh_prime,
            g_a: self.g_a,
            server_time: self.server_time,
            b,
            func,
        }
    }
}
