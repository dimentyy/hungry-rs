use rug::{integer, Integer};

use crate::{auth, crypto, tl};

use tl::mtproto::{funcs, types};
use tl::ser::Serialize;
use tl::{Int128, Int256};

pub struct ServerDhParams {
    pub(crate) nonce: Int128,
    pub(crate) server_nonce: Int128,
    pub(crate) new_nonce: Int256,
    pub(crate) tmp_aes_key: crypto::AesIgeKey,
    pub(crate) tmp_aes_iv: crypto::AesIgeIv,
    pub(crate) g: i32,
    pub(crate) dh_prime: Integer,
    pub(crate) g_a: Integer,
    pub(crate) server_time: i32,
}

impl ServerDhParams {
    #[inline]
    pub fn server_time(&self) -> i32 {
        self.server_time
    }

    pub fn set_client_dh_params(mut self, b: &[u8; 256], retry_id: i64) -> auth::SetClientDhParams {
        let one = Integer::from(1);

        let b = Integer::from_digits(b, integer::Order::MsfBe);

        // * g_b := pow(g, b) mod dh_prime
        let g_b = Integer::from(self.g).pow_mod(&b, &self.dh_prime).unwrap();

        // TODO: checks

        // * data := serialization client_DH_inner_data#6643b654 nonce:int128
        // server_nonce:int128 retry_id:long g_b:string = Client_DH_Inner_Data
        let client_dh_inner_data = tl::boxed(types::ClientDhInnerData {
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            retry_id,
            g_b: g_b.to_digits(integer::Order::MsfBe),
        });

        // TODO: random padding
        //
        // * data_with_hash := SHA1(data) + data + (0-15 random bytes);
        // such that length be divisible by 16;
        let mut data_with_hash = Vec::with_capacity(500);
        unsafe { data_with_hash.set_len(20) };

        client_dh_inner_data.serialize_into(&mut data_with_hash);

        let data_sha1 = crypto::sha1!(&data_with_hash[20..]);
        data_with_hash[..20].copy_from_slice(&data_sha1);

        unsafe { data_with_hash.set_len((data_with_hash.len() + 15) & !15) };

        // * encrypted_data := AES256_ige_encrypt(data_with_hash, tmp_aes_key, tmp_aes_iv);
        crypto::aes_ige_encrypt(&mut data_with_hash, &self.tmp_aes_key, &mut self.tmp_aes_iv);
        let encrypted_data = data_with_hash;

        let func = funcs::SetClientDhParams {
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            encrypted_data,
        };

        auth::SetClientDhParams {
            new_nonce: self.new_nonce,
            g: self.g,
            dh_prime: self.dh_prime,
            g_a: self.g_a,
            server_time: self.server_time,
            b,
            func,
        }
    }
}
