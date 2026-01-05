use std::{fmt, hash};

use crypto_bigint::modular::{MontyForm, MontyParams};
use crypto_bigint::{ConstOne, Odd, U2048};

use digest::Digest;

use crate::{crypto, tl};

use tl::{ConstSerializedLen, SerializedLen};

/// 64 lower-order bits of SHA1 (server_public_key);
/// the public key is represented as a bare type
/// `rsa_public_key n:string e:string = RSAPublicKey`,
/// where, as usual, n and e are numbers in
/// big endian format serialized as strings
/// of bytes, following which SHA1 is computed
///
/// ---
///
/// https://core.telegram.org/mtproto/auth_key#2-server-sends-response-of-the-form
pub type RsaKeyFingerprint = i64;

/// https://core.telegram.org/mtproto/auth_key#41-rsa-paddata-server-public-key-mentioned-above-is-implemented-as-follows
#[derive(Clone, Eq)]
pub struct RsaKey {
    n: Odd<U2048>,
    e: Odd<U2048>,
    fingerprint: RsaKeyFingerprint,
}

impl fmt::Debug for RsaKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RsaKey")
            .field("fingerprint", &format_args!("{:#018x}", self.fingerprint))
            .finish_non_exhaustive()
    }
}

impl fmt::Display for RsaKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RSA key [fingerprint={:#018x}, ..]", self.fingerprint)
    }
}

impl hash::Hash for RsaKey {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.fingerprint.hash(state);
    }
}

impl PartialEq for RsaKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.n == other.n && self.e == other.e
    }
}

impl RsaKey {
    fn compute_fingerprint(n: &Odd<U2048>, e: &Odd<U2048>) -> RsaKeyFingerprint {
        let n_bytes = n.to_be_bytes();
        let e_bytes = e.to_be_bytes();

        let n = crypto::trim_zeroes_left(&n_bytes);
        let e = crypto::trim_zeroes_left(&e_bytes);

        let n_len = n.serialized_len();
        let e_len = e.serialized_len();

        let mut buf = [0u8; <[u8; 256]>::SERIALIZED_LEN * 2];

        tl::ser::safe(n, &mut buf[..n_len]);
        tl::ser::safe(e, &mut buf[n_len..n_len + e_len]);

        let sha1 = sha1::Sha1::digest(&buf[..n_len + e_len]);

        i64::from_le_bytes(sha1[12..20].try_into().unwrap())
    }

    #[inline]
    #[must_use]
    #[track_caller]
    pub fn new(n: Odd<U2048>, e: Odd<U2048>) -> Self {
        assert!(n > e && e > Odd::ONE, "invalid public RSA key");

        let fingerprint = Self::compute_fingerprint(&n, &e);

        Self { n, e, fingerprint }
    }

    #[inline]
    #[must_use]
    pub const fn n(&self) -> &Odd<U2048> {
        &self.n
    }

    #[inline]
    #[must_use]
    pub const fn e(&self) -> &Odd<U2048> {
        &self.e
    }

    #[inline]
    #[must_use]
    pub const fn fingerprint(&self) -> RsaKeyFingerprint {
        self.fingerprint
    }

    /// * data_with_padding := data + random_padding_bytes;
    /// -- where random_padding_bytes are chosen so that the
    /// resulting length of data_with_padding is precisely 192 bytes,
    /// and data is the TL-serialized data to be encrypted as before.
    /// One has to check that data is not longer than 144 bytes.
    ///
    /// * a random 32-byte temp_key is generated.
    #[must_use]
    pub fn encrypted_data(
        &self,
        data_with_padding: &[u8; 192],
        temp_key: &[u8; 32],
    ) -> Option<U2048> {
        // * key_aes_encrypted := temp_key_xor + aes_encrypted;
        // -- exactly 256 bytes (2048 bits) long

        let mut key_aes_encrypted = [0; 256];

        let (temp_key_xor, data_with_hash) = key_aes_encrypted.split_at_mut(32);

        // * data_with_hash := data_pad_reversed + SHA256(temp_key + data_with_padding);
        // -- after this assignment, data_with_hash is exactly 224 bytes long.

        let (data_pad_reversed, hash) = data_with_hash.split_at_mut(192);

        // * data_pad_reversed := BYTE_REVERSE(data_with_padding);
        // -- is obtained from data_with_padding by reversing the byte order.
        for i in 0..192 {
            data_pad_reversed[i] = data_with_padding[192 - i - 1];
        }

        sha2::Sha256::new_with_prefix(temp_key)
            .chain_update(data_with_padding)
            .finalize_into(hash.try_into().unwrap());

        // * aes_encrypted := AES256_IGE(data_with_hash, temp_key, 0);
        // -- AES256-IGE encryption with zero IV.
        crypto::aes_ige_encrypt(data_with_hash, temp_key, &mut [0u8; 32]);
        let aes_encrypted = data_with_hash;

        // * temp_key_xor := temp_key XOR SHA256(aes_encrypted);
        // -- adjusted key, 32 bytes
        sha2::Sha256::new_with_prefix(aes_encrypted)
            .finalize_into(temp_key_xor.try_into().unwrap());

        for i in 0..32 {
            temp_key_xor[i] ^= temp_key[i];
        }

        let key_aes_encrypted = U2048::from_be_slice(&key_aes_encrypted);

        // * The value of key_aes_encrypted is compared with the RSA-modulus of
        // server_pubkey as a big-endian 2048-bit (256-byte) unsigned integer.
        if key_aes_encrypted >= self.n {
            // If key_aes_encrypted turns out to be greater than or equal to the RSA modulus,
            // the previous steps starting from the generation of new random temp_key are repeated.
            return None;
        }

        // Otherwise the final step is performed:

        // * encrypted_data := RSA(key_aes_encrypted, server_pubkey);
        // -- 256-byte big-endian integer is elevated to the requisite
        // power from the RSA public key modulo the RSA modulus, and
        // the result is stored as a big-endian integer consisting of
        // exactly 256 bytes (with leading zero bytes if required).
        let encrypted_data = MontyForm::new(&key_aes_encrypted, MontyParams::new(self.n))
            .pow(&self.e)
            .retrieve();

        Some(encrypted_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::common::hex;

    const N: U2048 = U2048::from_be_hex(
        "c8c11d635691fac091dd9489aedced2932aa8a0bcefef05fa800892d9b52ed03\
        200865c9e97211cb2ee6c7ae96d3fb0e15aeffd66019b44a08a240cfdd2868a8\
        5e1f54d6fa5deaa041f6941ddf302690d61dc476385c2fa655142353cb4e4b59\
        f6e5b6584db76fe8b1370263246c010c93d011014113ebdf987d093f9d37c2be\
        48352d69a1683f8f6e6c2167983c761e3ab169fde5daaa12123fa1beab621e4d\
        a5935e9c198f82f35eae583a99386d8110ea6bd1abb0f568759f62694419ea5f\
        69847c43462abef858b4cb5edc84e7b9226cd7bd7e183aa974a712c079dde85b\
        9dc063b8a5c08e8f859c0ee5dcd824c7807f20153361a7f63cfd2a433a1be7f5",
    );

    const E: U2048 = U2048::from_word(65537);

    const FINGERPRINT: i64 = -5595554452916591101;

    const DATA_WITH_PADDING: [u8; 192] = hex::decode(
        "955ff5a9081a8e635f5743de9b00000004453dc27100000004622f1fcb000000\
        f7a81627bbf511fa4afef71e94a0937474586c1add9198dda81a5df8393871c8\
        293623c5fb968894af1be7dfe9c7be813f9307789242fd0cb0c16a5cb39a8d3e\
        12270000635593b03fee033d0672f9afddf9124de9e77df6251806cba93482e4\
        c9e6e06e7d44e4c4baae821aff91af44789689faaee9bdfc7b2df8c08709afe5\
        7396c4638ceaa0dc30114f82447e81d3b53edc423b32660c43a5b8ad057b6450",
    );

    const TEMP_KEY: [u8; 32] =
        hex::decode("7dada0920c4973913229e0f881aec7b9db0c392d34f52fb0995ea493ecb4c09e");

    const ENCRYPTED_DATA: U2048 = U2048::from_be_hex(
        "b610642a828b4a61fe32931815cae318d311660580f1e0df768f3140f4d37dfc\
        fcac0c2870318de4ff2d2e0e9669bcfdc0bad06cadb1b59d9726b427368a9c7b\
        4fc0d5e7b2e99fc571968705c03acf5341fd7021bef653fa77b3776ae430e366\
        fc46d232459ebe128b08d80e049ae579a48b56ca93b520709468587c81af9666\
        6046e9ea85091d729e921e8d8a36f57b27644052dae7387c7f4131701d59cda7\
        5251dac66c94276280ef950d3c44c21e5a2454f7da7a6818cf23ae9c490b72b2\
        170d7cbc24f8a93db739d76f2d241c78b80123faaff3e664f074d6375d794dbf\
        2800a0b5bb48d54eceafedfb355bfbebd287d9023264e3b53627888250787a9e",
    );

    #[test]
    fn test_crypto_rsa() {
        let n = Odd::new(N).unwrap();
        let e = Odd::new(E).unwrap();

        let key = RsaKey::new(n, e);

        assert_eq!(key.fingerprint(), FINGERPRINT);

        let encrypted_data = key.encrypted_data(&DATA_WITH_PADDING, &TEMP_KEY).unwrap();

        assert_eq!(encrypted_data, ENCRYPTED_DATA);
    }
}
