use aes::Aes256;
use cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};

pub type AesIgeKey = [u8; 32];
pub type AesIgeIv = [u8; 32];

/// Decrypts a `buffer` using AES-256-IGE algorithm in-place.
///
/// # Panics
///
/// * If the `buffer` length in bytes is not a multiple of 16.
#[track_caller]
pub fn aes_ige_decrypt(buffer: &mut [u8], key: &AesIgeKey, iv: &mut AesIgeIv) {
    assert!(buffer.len().is_multiple_of(16));

    let cipher = Aes256::new(key.into());

    let (c, p) = iv.split_at_mut(16);
    let c: &mut [u8; 16] = c.try_into().unwrap();
    let p: &mut [u8; 16] = p.try_into().unwrap();

    for block in buffer.chunks_exact_mut(16) {
        let block: &mut [u8; 16] = block.try_into().unwrap();

        for i in 0..16 {
            p[i] ^= block[i]
        }

        cipher.decrypt_block(p.into());

        for i in 0..16 {
            p[i] ^= c[i]
        }

        *c = *block;
        *block = *p;
    }
}

/// Encrypts a `buffer` using AES-256-IGE algorithm in-place.
///
/// # Panics
///
/// * If the `buffer` length in bytes is not a multiple of 16.
#[track_caller]
pub fn aes_ige_encrypt(buffer: &mut [u8], key: &AesIgeKey, iv: &mut AesIgeIv) {
    assert!(buffer.len().is_multiple_of(16));

    let cipher = Aes256::new(key.into());

    let (c, p) = iv.split_at_mut(16);
    let c: &mut [u8; 16] = c.try_into().unwrap();
    let p: &mut [u8; 16] = p.try_into().unwrap();

    for block in buffer.chunks_exact_mut(16) {
        let block: &mut [u8; 16] = block.try_into().unwrap();

        for i in 0..16 {
            c[i] ^= block[i]
        }

        cipher.encrypt_block(c.into());

        for i in 0..16 {
            c[i] ^= p[i]
        }

        *p = *block;
        *block = *c;
    }
}
