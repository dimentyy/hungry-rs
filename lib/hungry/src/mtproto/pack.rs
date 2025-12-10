use bytes::BytesMut;

use crate::crypto;
use crate::mtproto::{AuthKey, DecryptedMessage, EncryptedMessage, Envelope, PlainEnvelope, Side};
use crate::utils::SliceExt;

pub fn pack_plain(buffer: &mut BytesMut, mut envelope: PlainEnvelope, message_id: i64) {
    let excess = envelope.adapt(buffer);
    let (h, _) = envelope.buffers();

    h[0..8].fill(0); // auth_key_id
    h[8..16].copy_from_slice(&message_id.to_le_bytes());

    let length = buffer.len() as i32;
    h[16..20].copy_from_slice(&length.to_le_bytes());

    envelope.unsplit(buffer, excess);
}

pub fn pack_encrypted(
    buffer: &mut BytesMut,
    mut envelope: Envelope,
    auth_key: &AuthKey,
    message: &DecryptedMessage,
) {
    let excess = envelope.adapt(buffer);
    let (h, f) = envelope.buffers();

    let plaintext_len = buffer.len();

    // TODO: allow custom padding length; currently minimum possible
    let random_padding_len = (20 - (plaintext_len % 16)) % 16 + 12; // 12..28
    let random_padding = &mut f[..random_padding_len];
    getrandom::fill(random_padding).unwrap();

    let plaintext_header = h[EncryptedMessage::HEADER_LEN..].arr_mut();

    plaintext_header[0..8].copy_from_slice(&message.salt.to_le_bytes());
    plaintext_header[8..16].copy_from_slice(&message.session_id.to_le_bytes());

    let msg_key = auth_key.compute_msg_key(plaintext_header, buffer, random_padding, Side::Client);

    h[0..8].copy_from_slice(auth_key.id());
    h[8..24].copy_from_slice(&msg_key);

    envelope.unsplit(buffer, excess);

    buffer.truncate(
        EncryptedMessage::HEADER_LEN
            + DecryptedMessage::HEADER_LEN
            + plaintext_len
            + random_padding_len,
    );

    let (aes_key, aes_iv) = auth_key.compute_aes_params(&msg_key, Side::Client);

    crypto::aes_ige_encrypt(
        &mut buffer[EncryptedMessage::HEADER_LEN..],
        &aes_key,
        &aes_iv,
    );
}
