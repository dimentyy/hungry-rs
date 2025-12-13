// STATUS: stable.

use std::ops::RangeTo;

use rug::{integer::Order::MsfBe, Integer};

use crate::utils::SliceExt;
use crate::{crypto, tl};

/// https://core.telegram.org/mtproto/auth_key#41-rsa-paddata-server-public-key-mentioned-above-is-implemented-as-follows
#[must_use]
#[derive(Clone, Debug)]
pub struct RsaKey {
    n: Integer,
    e: Integer,
    fingerprint: i64,
}

impl RsaKey {
    #[must_use]
    pub fn calculate_fingerprint(n: &Integer, e: &Integer) -> i64 {
        let n_len = n.significant_digits::<u8>();
        let e_len = e.significant_digits::<u8>();

        if n <= e || e <= &1 || n.is_even() || e.is_even() || n_len != 256 || e_len > 256 {
            panic!("invalid public RSA key")
        }

        let n_ser_len = tl::ser::bytes_len(n_len);
        let e_ser_len = tl::ser::bytes_len(e_len);

        let mut buf = [0; tl::ser::bytes_len(256) * 2];

        let (bytes, extra) = tl::ser::prepare_bytes(&mut buf, n_len);
        n.write_digits(bytes, MsfBe);

        let (bytes, _) = tl::ser::prepare_bytes(extra, e_len);
        e.write_digits(bytes, MsfBe);

        let sha1 = crypto::sha1!(&buf[..n_ser_len + e_ser_len]);

        i64::from_le_bytes(*sha1[12..].arr())
    }

    #[inline]
    pub fn new(n: Integer, e: Integer) -> Self {
        let fingerprint = Self::calculate_fingerprint(&n, &e);

        Self { n, e, fingerprint }
    }

    #[inline]
    #[must_use]
    pub fn fingerprint(&self) -> i64 {
        self.fingerprint
    }

    /// * data_with_padding := data + random_padding_bytes;
    ///
    /// -- where random_padding_bytes are chosen so that the resulting
    /// length of data_with_padding is precisely 192 bytes, and
    /// data is the TL-serialized data to be encrypted as before.
    /// One has to check that data is not longer than 144 bytes.
    ///
    /// * data_pad_reversed := BYTE_REVERSE(data_with_padding);
    ///
    /// -- is obtained from data_with_padding by reversing the byte order.
    ///
    /// * a random 32-byte temp_key is generated.
    #[must_use]
    pub fn key_aes_encrypted(
        &self,
        data_with_padding: &[u8; 192],
        data_pad_reversed: &[u8; 192],
        temp_key: &[u8; 32],
        key_aes_encrypted: &mut [u8; 256],
    ) -> bool {
        // * data_with_hash := data_pad_reversed + SHA256(temp_key + data_with_padding);
        // -- after this assignment, data_with_hash is exactly 224 bytes long.
        let mut data_with_hash = [0; 224];
        data_with_hash[..192].copy_from_slice(data_pad_reversed);
        data_with_hash[192..].copy_from_slice(&crypto::sha256!(&temp_key, data_with_padding));

        // * aes_encrypted := AES256_IGE(data_with_hash, temp_key, 0);
        // -- AES256-IGE encryption with zero IV.
        crypto::aes_ige_encrypt(&mut data_with_hash, temp_key, &mut [0u8; 32]);
        let aes_encrypted = data_with_hash;

        // * temp_key_xor := temp_key XOR SHA256(aes_encrypted);
        // -- adjusted key, 32 bytes
        let mut temp_key_xor = crypto::sha256!(&aes_encrypted);

        for i in 0..32 {
            temp_key_xor[i] ^= temp_key[i];
        }

        // * key_aes_encrypted := temp_key_xor + aes_encrypted;
        // -- exactly 256 bytes (2048 bits) long
        key_aes_encrypted[..32].copy_from_slice(&temp_key_xor);
        key_aes_encrypted[32..].copy_from_slice(&aes_encrypted);

        // * The value of key_aes_encrypted is compared with the RSA-modulus of
        // server_pubkey as a big-endian 2048-bit (256-byte) unsigned integer.
        if Integer::from_digits(key_aes_encrypted, MsfBe) >= self.n {
            // If key_aes_encrypted turns out to be greater than or equal to the RSA modulus,
            // the previous steps starting from the generation of new random temp_key are repeated.
            return false;
        }

        // Otherwise the final step is performed:
        true
    }

    /// * encrypted_data := RSA(key_aes_encrypted, server_pubkey);
    ///
    /// -- 256-byte big-endian integer is elevated to the requisite power from the
    /// RSA public key modulo the RSA modulus, and the result is stored as a big-endian
    /// integer consisting of exactly 256 bytes (with leading zero bytes if required).
    #[must_use = "leading bytes of `encrypted_data` must be zeroed in the returned range"]
    pub fn encrypted_data(
        &self,
        key_aes_encrypted: &[u8; 256],
        encrypted_data: &mut [u8; 256],
    ) -> RangeTo<usize> {
        let key_aes_encrypted = Integer::from_digits(key_aes_encrypted, MsfBe);

        let result = key_aes_encrypted.pow_mod(&self.e, &self.n).unwrap();

        let index = 256 - result.significant_digits::<u8>();

        result.write_digits(&mut encrypted_data[index..], MsfBe);

        ..index
    }
}

/// TODO: reimplement hex.
#[cfg(test)]
mod tests {
    use super::*;

    const N: &str = "253428894488404155649716895907134732068988477590847790525820265945460224638539\
    40585885215951168491965708222649399180603818074200620463776135424884632162512403163793083921641\
    63156474095952941935959585294116684894058595233761333302239609658411795489221603122923730294370\
    18775884567383353986024616752250817918203931537575049526362349513232378200365435810478269061209\
    27972487366805292115792231423684261262330394324750785450942589751755390156647751460719351439969\
    05994956961530280905072150033023900507788985532391750994825572208164468944212729760542257970714\
    2646660768825302832201908302295573257427896031830742328565032949";

    const FINGERPRINT: i64 = -5595554452916591101;

    const DATA_WITH_PADDING: &str = "955ff5a9081a8e635f5743de9b00000004453dc27100000004622f1fcb0000\
    00f7a81627bbf511fa4afef71e94a0937474586c1add9198dda81a5df8393871c8293623c5fb968894af1be7dfe9c7b\
    e813f9307789242fd0cb0c16a5cb39a8d3e12270000635593b03fee033d0672f9afddf9124de9e77df6251806cba934\
    82e4c9e6e06e7d44e4c4baae821aff91af44789689faaee9bdfc7b2df8c08709afe57396c4638ceaa0dc30114f82447\
    e81d3b53edc423b32660c43a5b8ad057b6450";

    const TEMP_KEY: &str = "7dada0920c4973913229e0f881aec7b9db0c392d34f52fb0995ea493ecb4c09e";

    const ENCRYPTED_DATA: &str = "b610642a828b4a61fe32931815cae318d311660580f1e0df768f3140f4d37dfcf\
    cac0c2870318de4ff2d2e0e9669bcfdc0bad06cadb1b59d9726b427368a9c7b4fc0d5e7b2e99fc571968705c03acf53\
    41fd7021bef653fa77b3776ae430e366fc46d232459ebe128b08d80e049ae579a48b56ca93b520709468587c81af966\
    66046e9ea85091d729e921e8d8a36f57b27644052dae7387c7f4131701d59cda75251dac66c94276280ef950d3c44c2\
    1e5a2454f7da7a6818cf23ae9c490b72b2170d7cbc24f8a93db739d76f2d241c78b80123faaff3e664f074d6375d794\
    dbf2800a0b5bb48d54eceafedfb355bfbebd287d9023264e3b53627888250787a9e";

    #[test]
    fn test_crypto_rsa() {
        let n = Integer::from_str_radix(N, 10).unwrap();
        let e = Integer::from(65537);

        let key = RsaKey::new(n, e);

        assert_eq!(key.fingerprint(), FINGERPRINT);

        let mut data_with_padding = [0; 192];
        hex::decode_to_slice(DATA_WITH_PADDING, &mut data_with_padding).unwrap();

        let mut data_pad_reversed = data_with_padding;
        data_pad_reversed.reverse();

        let mut temp_key = [0; 32];
        hex::decode_to_slice(TEMP_KEY, &mut temp_key).unwrap();

        let mut key_aes_encrypted = [0u8; 256];

        let success = key.key_aes_encrypted(
            &data_with_padding,
            &data_pad_reversed,
            &temp_key,
            &mut key_aes_encrypted,
        );

        assert!(success);

        let mut encrypted_data = [0u8; 256];

        let range = key.encrypted_data(&key_aes_encrypted, &mut encrypted_data);

        assert_eq!(range, ..0);

        let mut expected = [0u8; 256];
        hex::decode_to_slice(ENCRYPTED_DATA, &mut expected).unwrap();

        assert_eq!(encrypted_data.as_ref(), &expected);
    }
}
