use crate::{auth, crypto, tl};
use bytes::BytesMut;
use hungry_tl::SerializedLen;
use rug::{Integer, integer::Order::MsfBe};

use tl::mtproto::{funcs, types};
use tl::ser::SerializeInto;
use tl::{Int128, Int256};

#[must_use]
pub struct ServerDhParamsOk {
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

impl ServerDhParamsOk {
    #[inline]
    pub fn server_time(&self) -> i32 {
        self.server_time
    }

    pub fn set_client_dh_params(mut self, b: &[u8; 256], retry_id: i64) -> auth::SetClientDhParams {
        let one = Integer::from(1);

        let b = Integer::from_digits(b, MsfBe);

        // * g_b := pow(g, b) mod dh_prime
        let g_b = Integer::from(self.g).pow_mod(&b, &self.dh_prime).unwrap();

        // TODO: checks

        // * data := serialization client_DH_inner_data#6643b654 nonce:int128
        // server_nonce:int128 retry_id:long g_b:string = Client_DH_Inner_Data
        let client_dh_inner_data = tl::boxed(types::ClientDhInnerData {
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            retry_id,
            g_b: g_b.to_digits(MsfBe),
        });

        let serialized_len = client_dh_inner_data.serialized_len();

        // * data_with_hash := SHA1(data) + data + (0-15 random bytes);
        // such that length be divisible by 16;
        let mut data_with_hash = Vec::with_capacity((20 + serialized_len + 15) & !15);

        // SAFETY: uninitialized data is not read.
        unsafe { data_with_hash.set_len(20) };

        data_with_hash.ser(&client_dh_inner_data);

        let data_sha1 = crypto::sha1!(&data_with_hash[20..]);
        data_with_hash[..20].copy_from_slice(&data_sha1);

        // TODO: allow custom random padding.
        getrandom::fill_uninit(data_with_hash.spare_capacity_mut()).unwrap();

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
