//! TODO: security checks, retries

use rug::{integer, Integer};

use crate::utils::SliceExt;
use crate::{crypto, mtproto, tl};

use tl::mtproto::{enums, funcs, types};
use tl::ser::Serialize;
use tl::{Int128, Int256};

#[inline]
pub fn start(nonce: Int128) -> ReqPqMulti {
    let func = funcs::ReqPqMulti { nonce };

    ReqPqMulti { func }
}

pub struct ReqPqMulti {
    func: funcs::ReqPqMulti,
}

impl ReqPqMulti {
    #[inline]
    pub fn func(&self) -> &funcs::ReqPqMulti {
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

        fn without_leading_zeros(i: u64) -> Vec<u8> {
            let bytes = i.to_be_bytes();

            let index = bytes.iter().position(|&x| x != 0).unwrap_or(bytes.len());

            bytes[index..].to_vec()
        }

        ResPq {
            nonce: self.func.nonce,
            server_nonce: res_pq.server_nonce,
            server_public_key_fingerprints: res_pq.server_public_key_fingerprints,
            pq: res_pq.pq,
            p: without_leading_zeros(p),
            q: without_leading_zeros(q),
        }
    }
}

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

    pub fn req_dh_params(
        self,
        mut random_padding_bytes: [u8; 192],
        new_nonce: Int256,
        key: &'_ crypto::RsaKey,
    ) -> ReqDhParams<'_> {
        let public_key_fingerprint = key.fingerprint();

        if !self
            .server_public_key_fingerprints
            .contains(&public_key_fingerprint)
        {
            todo!()
        }

        let pq_inner_data = tl::boxed(dbg!(types::PQInnerData {
            pq: self.pq.clone(),
            p: self.p.clone(),
            q: self.q.clone(),
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            new_nonce,
        }));

        pq_inner_data.serialize_into(&mut random_padding_bytes);

        let data_with_padding = random_padding_bytes;

        let mut data_pad_reversed = data_with_padding;
        data_pad_reversed.reverse();

        let mut encrypted_data = Vec::with_capacity(256);
        unsafe { encrypted_data.set_len(256) };

        let func = funcs::ReqDhParams {
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            p: self.p,
            q: self.q,
            public_key_fingerprint,
            encrypted_data,
        };

        ReqDhParams {
            data_with_padding,
            data_pad_reversed,
            new_nonce,
            key,
            func,
        }
    }
}

pub struct ReqDhParams<'a> {
    data_with_padding: [u8; 192],
    data_pad_reversed: [u8; 192],
    new_nonce: Int256,
    key: &'a crypto::RsaKey,
    func: funcs::ReqDhParams,
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

        let len = self.key.encrypted_data(key_aes_encrypted, encrypted_data);
        encrypted_data[..256 - len].fill(0);

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

    pub fn server_dh_params(self, server_dh_params: enums::ServerDhParams) -> ServerDhParams {
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
        crypto::aes_ige_decrypt(encrypted_answer, &tmp_aes_key, &tmp_aes_iv);
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

        ServerDhParams {
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

pub struct ServerDhParams {
    nonce: Int128,
    server_nonce: Int128,
    new_nonce: Int256,
    tmp_aes_key: crypto::AesIgeKey,
    tmp_aes_iv: crypto::AesIgeIv,
    g: i32,
    dh_prime: Integer,
    g_a: Integer,
    server_time: i32,
}

impl ServerDhParams {
    #[inline]
    pub fn server_time(&self) -> i32 {
        self.server_time
    }

    pub fn set_client_dh_params(self, b: &[u8; 256], retry_id: i64) -> SetClientDhParams {
        let one = Integer::from(1);

        crate::utils::dump(b, "b").unwrap();

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
        crypto::aes_ige_encrypt(&mut data_with_hash, &self.tmp_aes_key, &self.tmp_aes_iv);
        let encrypted_data = data_with_hash;

        let func = funcs::SetClientDhParams {
            nonce: self.nonce,
            server_nonce: self.server_nonce,
            encrypted_data,
        };

        SetClientDhParams {
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

pub struct SetClientDhParams {
    new_nonce: Int256,
    g: i32,
    dh_prime: Integer,
    g_a: Integer,
    server_time: i32,
    b: Integer,
    func: funcs::SetClientDhParams,
}

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

    pub fn dh_gen_ok(self, dh_gen_ok: types::DhGenOk) -> (mtproto::AuthKey, i64) {
        if dh_gen_ok.nonce != self.func.nonce {
            todo!()
        }

        if dh_gen_ok.server_nonce != self.func.server_nonce {
            todo!()
        }

        let mut data = [0; 256];

        let g_ab = self.g_a.pow_mod(&self.b, &self.dh_prime).unwrap();

        let len = g_ab.significant_digits::<u8>();

        g_ab.write_digits(&mut data[256 - len..], integer::Order::MsfBe);

        let auth_key = mtproto::AuthKey::new(data);

        if dh_gen_ok.new_nonce_hash_1 != new_nonce_hash(&auth_key, &self.new_nonce, 1) {
            todo!()
        }

        let mut salt = i64::from_le_bytes(*self.new_nonce[..8].arr())
            ^ i64::from_le_bytes(*self.func.server_nonce[..8].arr());

        (auth_key, salt)
    }
}
