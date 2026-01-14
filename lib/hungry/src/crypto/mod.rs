#![forbid(unsafe_code, clippy::todo)]

mod aes;
mod rsa;

pub use aes::{AesIgeIv, AesIgeKey, aes_ige_decrypt, aes_ige_encrypt};
pub use rsa::{RsaKey, RsaKeyFingerprint};

#[must_use]
pub fn trim_zeroes_left(x: &[u8]) -> &[u8] {
    let Some(pos) = x.iter().position(|x| *x != 0) else {
        return &[0];
    };

    &x[pos..]
}

#[must_use]
pub fn factorize_pq(pq: u64) -> Option<(u64, u64)> {
    let mut map = num_prime::nt_funcs::factorize64(pq);

    if map.len() != 2 {
        return None;
    }

    let (Some((p, 1)), Some((q, 1))) = (map.pop_first(), map.pop_last()) else {
        return None;
    };

    Some((p, q))
}
