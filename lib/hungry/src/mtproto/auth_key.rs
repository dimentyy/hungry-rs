use std::fmt;

use crate::utils::SliceExt;
use crate::{crypto, mtproto};

#[derive(Clone)]
pub struct AuthKey {
    data: [u8; 256],

    aux_hash: [u8; 8],
    id: [u8; 8],
}

impl fmt::Display for AuthKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "auth key {:#016x}", u64::from_ne_bytes(self.id))
    }
}

impl fmt::Debug for AuthKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = u64::from_ne_bytes(self.id);

        f.debug_struct("AuthKey")
            .field("id", &format_args!("{:#016x}", id))
            .finish()
    }
}

/// A 2048-bit key shared by the client device and the server,
/// created upon user registration directly on the client device by
/// exchanging Diffie-Hellman keys, and never transmitted over a network.
///
/// https://core.telegram.org/mtproto/description#authorization-key-auth-key
impl AuthKey {
    /// Create a new instance of [`AuthKey`] from its data.
    pub fn new(data: [u8; 256]) -> Self {
        let hash = crypto::sha1!(&data);

        let aux_hash = *hash[..8].arr();
        let id = *hash[12..].arr();

        Self { data, aux_hash, id }
    }

    /// Actual underlying data used for cryptographic operations.
    #[inline]
    pub fn data(&self) -> &[u8; 256] {
        &self.data
    }

    /// Consume [`AuthKey`] returning its owned underling data.
    pub fn into_inner(self) -> [u8; 256] {
        self.data
    }

    /// The 64 higher-order bits of the SHA1 hash of the authorization key.
    /// It must not be confused with auth_key_hash during the key exchange.
    ///
    /// https://core.telegram.org/mtproto/auth_key#9-server-responds-in-one-of-three-ways
    #[inline]
    pub fn aux_hash(&self) -> &[u8; 8] {
        &self.aux_hash
    }

    /// The 64 lower-order bits of the SHA1 hash of the authorization key. \
    ///
    /// https://core.telegram.org/mtproto/description#key-identifier-auth-key-id
    #[inline]
    pub fn id(&self) -> &[u8; 8] {
        &self.id
    }

    /// Compute msg_key.
    ///
    /// https://core.telegram.org/mtproto/description#message-key-msg-key \
    /// https://core.telegram.org/mtproto/description#defining-aes-key-and-initialization-vector
    pub(super) fn msg_key(&self, buffer: &[u8], padding: &[u8], side: mtproto::Side) -> [u8; 16] {
        let x = side.x();

        // SHA256(substr(auth_key, 88 + x, 32) + plaintext + random_padding);
        let msg_key_large = crypto::sha256!(&self.data[88 + x..88 + x + 32], buffer, padding);

        // msg_key = substr(msg_key_large, 8, 16);
        let msg_key = *msg_key_large[8..24].arr();

        msg_key
    }

    /// Compute aes_key, aes_iv.
    ///
    /// https://core.telegram.org/mtproto/description#defining-aes-key-and-initialization-vector
    pub(super) fn compute(&self, msg_key: &[u8; 16], side: mtproto::Side) -> ([u8; 32], [u8; 32]) {
        let x = side.x();

        // sha256_a = SHA256(msg_key + substr(auth_key, x, 36));
        let sha256_a = crypto::sha256!(msg_key, &self.data[x..x + 36]);

        // sha256_b = SHA256(substr(auth_key, 40 + x, 36) + msg_key);
        let sha256_b = crypto::sha256!(&self.data[40 + x..40 + x + 36], msg_key);

        // aes_key = substr(sha256_a, 0, 8) + substr(sha256_b, 8, 16) + substr(sha256_a, 24, 8);
        let mut aes_key = [0u8; 32];

        aes_key[0..8].copy_from_slice(&sha256_a[0..8]);
        aes_key[8..24].copy_from_slice(&sha256_b[8..24]);
        aes_key[24..32].copy_from_slice(&sha256_a[24..32]);

        // aes_iv = substr(sha256_b, 0, 8) + substr(sha256_a, 8, 16) + substr(sha256_b, 24, 8);
        let mut aes_iv = [0; 32];

        aes_iv[0..8].copy_from_slice(&sha256_b[0..8]);
        aes_iv[8..24].copy_from_slice(&sha256_a[8..24]);
        aes_iv[24..32].copy_from_slice(&sha256_b[24..32]);

        (aes_key, aes_iv)
    }
}
