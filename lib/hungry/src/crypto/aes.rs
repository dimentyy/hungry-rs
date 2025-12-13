// STATUS: stable.

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes256;

pub type AesIgeKey = [u8; 32];
pub type AesIgeIv = [u8; 32];

pub fn aes_ige_decrypt(buffer: &mut [u8], key: &AesIgeKey, iv: &mut AesIgeIv) {
    assert!(buffer.len().is_multiple_of(16));

    let cipher = Aes256::new(GenericArray::from_slice(key));

    let (iv1, iv2) = iv.split_at_mut(16);

    let mut next_iv1 = [0u8; 16];

    for block in buffer.chunks_mut(16) {
        // next iv1 = block (ciphertext)
        next_iv1.copy_from_slice(block);

        // block (ciphertext) XOR iv2 (previous plaintext)
        for i in 0..16 {
            block[i] ^= iv2[i]
        }

        cipher.decrypt_block(GenericArray::from_mut_slice(block));

        // block (plaintext) XOR iv1 (previous ciphertext)
        for i in 0..16 {
            block[i] ^= iv1[i]
        }

        // iv1 = next iv1 (ciphertext)
        iv1.copy_from_slice(&next_iv1);

        // iv2 = block (plaintext)
        iv2.copy_from_slice(block);
    }
}

pub fn aes_ige_encrypt(buffer: &mut [u8], key: &AesIgeKey, iv: &mut AesIgeIv) {
    assert!(buffer.len().is_multiple_of(16));

    let cipher = Aes256::new(GenericArray::from_slice(key));

    let (iv1, iv2) = iv.split_at_mut(16);

    let mut next_iv2 = [0u8; 16];

    for block in buffer.chunks_mut(16) {
        // next iv2 = block (plaintext)
        next_iv2.copy_from_slice(block);

        // block (plaintext) XOR iv1 (previous ciphertext)
        for i in 0..16 {
            block[i] ^= iv1[i]
        }

        cipher.encrypt_block(GenericArray::from_mut_slice(block));

        // block (ciphertext) XOR iv2 (previous plaintext)
        for i in 0..16 {
            block[i] ^= iv2[i]
        }

        // iv1 = block (ciphertext)
        iv1.copy_from_slice(block);

        // iv2 = next iv2 (plaintext)
        iv2.copy_from_slice(&next_iv2);
    }
}
