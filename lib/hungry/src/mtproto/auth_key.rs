// STATUS: stable.

use std::fmt;

use crate::{crypto, mtproto};

/// The middle 128 bits of the SHA-256 hash of the message to be encrypted
/// (including the internal header and the padding bytes for MTProto),
/// prepended by a 32-byte fragment of the authorization key.
///
/// ---
/// https://core.telegram.org/mtproto/description#message-key-msg-key
pub type MsgKey = crate::tl::Int128;

/// A 2048-bit key shared by the client device and the server,
/// created upon user registration directly on the client device by
/// exchanging Diffie-Hellman keys, and never transmitted over a network.
///
/// ---
/// https://core.telegram.org/mtproto/description#authorization-key-auth-key
#[must_use]
#[derive(Clone)]
#[repr(align(8))]
pub struct AuthKey {
    data: [u8; 256],

    aux_hash: [u8; 8],
    id: [u8; 8],
}

impl fmt::Display for AuthKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = u64::from_ne_bytes(self.id);

        write!(f, "auth key [id={id:#016x}, ..]")
    }
}

impl fmt::Debug for AuthKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = u64::from_ne_bytes(self.id);

        f.debug_struct("AuthKey")
            .field("id", &format_args!("{id:#016x}"))
            .finish_non_exhaustive()
    }
}

impl AuthKey {
    /// Create a new instance of [`AuthKey`] from its data.
    pub fn new(data: [u8; 256]) -> Self {
        let hash = crypto::sha1!(&data);

        let aux_hash = hash[0..8].try_into().unwrap();
        let id = hash[12..20].try_into().unwrap();

        Self { data, aux_hash, id }
    }

    /// Actual underlying data used for cryptographic operations.
    #[must_use]
    #[inline(always)]
    pub fn data(&self) -> &[u8; 256] {
        &self.data
    }

    /// Consume the [`AuthKey`] returning its owned underling data.
    #[must_use]
    #[inline(always)]
    pub fn into_inner(self) -> [u8; 256] {
        self.data
    }

    /// The 64 higher-order bits of the SHA1 hash of the authorization key.
    /// It must not be confused with auth_key_hash during the key exchange.
    ///
    /// ---
    /// https://core.telegram.org/mtproto/auth_key#9-server-responds-in-one-of-three-ways
    #[must_use]
    #[inline(always)]
    pub fn aux_hash(&self) -> &[u8; 8] {
        &self.aux_hash
    }

    /// The 64 lower-order bits of the SHA1 hash of the authorization key.
    ///
    /// ---
    /// https://core.telegram.org/mtproto/description#key-identifier-auth-key-id
    #[must_use]
    #[inline(always)]
    pub fn id(&self) -> &[u8; 8] {
        &self.id
    }

    /// Compute [`MsgKey`].
    ///
    /// ---
    /// https://core.telegram.org/mtproto/description#defining-aes-key-and-initialization-vector
    #[must_use]
    pub fn compute_msg_key(&self, plaintext: &[u8], side: mtproto::Side) -> MsgKey {
        let x = side.x();

        // * msg_key_large = SHA256(substr(auth_key, 88 + x, 32) + plaintext + random_padding);
        let msg_key_large = crypto::sha256!(&self.data[88 + x..88 + x + 32], plaintext);

        // * msg_key = substr(msg_key_large, 8, 16);
        msg_key_large[8..24].try_into().unwrap()
    }

    /// Compute [`AesIgeKey`] and [`AesIgeIv`].
    ///
    /// ---
    /// https://core.telegram.org/mtproto/description#defining-aes-key-and-initialization-vector
    ///
    /// [`AesIgeKey`]: crypto::AesIgeKey
    /// [`AesIgeIv`]: crypto::AesIgeIv
    #[must_use]
    pub fn compute_aes_params(
        &self,
        msg_key: &MsgKey,
        side: mtproto::Side,
    ) -> (crypto::AesIgeKey, crypto::AesIgeIv) {
        let x = side.x();

        // * sha256_a = SHA256(msg_key + substr(auth_key, x, 36));
        let mut sha256_a = crypto::sha256!(msg_key, &self.data[x..x + 36]);

        // * sha256_b = SHA256(substr(auth_key, 40 + x, 36) + msg_key);
        let mut sha256_b = crypto::sha256!(&self.data[40 + x..40 + x + 36], msg_key);

        // In-place slice swap instead of a substitution.
        sha256_a[8..8 + 16].swap_with_slice(&mut sha256_b[8..8 + 16]);

        // * aes_key = substr(sha256_a, 0, 8) + substr(sha256_b, 8, 16) + substr(sha256_a, 24, 8);
        // * aes_iv = substr(sha256_b, 0, 8) + substr(sha256_a, 8, 16) + substr(sha256_b, 24, 8);
        let aes_key = sha256_a.into();
        let aes_iv = sha256_b.into();

        (aes_key, aes_iv)
    }
}
