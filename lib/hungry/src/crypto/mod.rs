#![forbid(unsafe_code)]

mod aes;
mod rsa;

pub use aes::{AesIgeIv, AesIgeKey, aes_ige_decrypt, aes_ige_encrypt};
pub use rsa::{RsaKey, RsaKeyFingerprint};

pub fn trim_zeroes_left(x: &[u8]) -> &[u8] {
    let Some(pos) = x.iter().position(|&x| x != 0) else {
        return &[0];
    };

    &x[pos..]
}
