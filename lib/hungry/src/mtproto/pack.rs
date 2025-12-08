use bytes::BytesMut;

use crate::crypto;
use crate::mtproto::{AuthKey, DecryptedMessage, EncryptedMessage, Envelope, PlainEnvelope, Side};

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
    salt: i64,
    session_id: i64,
) {
    let excess = envelope.adapt(buffer);
    let (h, f) = envelope.buffers();

    let payload_len = buffer.len();

    // TODO: allow custom padding length; currently minimum possible
    let padding_len = (20 - (payload_len % 16)) % 16 + 12; // 12..28

    getrandom::fill(&mut f[..padding_len]).unwrap();

    let msg_key = auth_key.compute_msg_key(buffer, &f[..padding_len], Side::Client);

    let (aes_key, aes_iv) = auth_key.compute_aes_params(&msg_key, Side::Client);

    h[0..8].copy_from_slice(auth_key.id());
    h[8..24].copy_from_slice(&msg_key);

    h[24..32].copy_from_slice(&salt.to_le_bytes());
    h[32..40].copy_from_slice(&session_id.to_le_bytes());

    envelope.unsplit(buffer, excess);

    buffer.truncate(
        EncryptedMessage::HEADER_LEN + DecryptedMessage::HEADER_LEN + payload_len + padding_len,
    );

    crypto::aes_ige_encrypt(&mut buffer[8 + 16..], &aes_key, &aes_iv);
}
