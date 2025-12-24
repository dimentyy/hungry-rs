use bytes::BytesMut;

use crate::mtproto::{
    AuthKey, DecryptedMessage, EncryptedEnvelope, EncryptedMessage, Msg, PlainEnvelope, Side,
};

use crate::tl::ser::SerializeUnchecked;

pub fn pack_plain(mut envelope: PlainEnvelope, buffer: &mut BytesMut, message_id: i64) {
    let excess = envelope.adapt(buffer);
    let (h, _) = envelope.buffers();

    unsafe {
        let mut buf = std::ptr::NonNull::new_unchecked(h.as_mut_ptr());

        buf = 0i64.serialize_unchecked(buf); // auth_key_id
        buf = message_id.serialize_unchecked(buf);
        (buffer.len() as i32).serialize_unchecked(buf); // message_data_length
    }

    envelope.unsplit(buffer, excess);
}

pub fn pack_encrypted(
    mut envelope: EncryptedEnvelope,
    buffer: &mut BytesMut,
    auth_key: &AuthKey,
    message: DecryptedMessage,
    msg: Msg,
) {
    let excess = envelope.adapt(buffer);
    let (h, f) = envelope.buffers();

    let plaintext_len = buffer.len();

    // TODO: allow custom padding length; currently minimum possible
    let random_padding_len = (20 - (plaintext_len % 16)) % 16 + 12; // 12..28
    let random_padding = &mut f[..random_padding_len];
    getrandom::fill(random_padding).unwrap();

    unsafe {
        let mut buf =
            std::ptr::NonNull::new_unchecked(h.as_mut_ptr()).add(EncryptedMessage::HEADER_LEN);

        buf = message.salt.serialize_unchecked(buf);
        buf = message.session_id.serialize_unchecked(buf);

        buf = msg.msg_id.serialize_unchecked(buf);
        buf = msg.seq_no.serialize_unchecked(buf);

        (plaintext_len as i32).serialize_unchecked(buf);
    }

    envelope.unsplit(buffer, excess);

    buffer.truncate(
        EncryptedMessage::HEADER_LEN
            + DecryptedMessage::HEADER_LEN
            + Msg::HEADER_LEN
            + plaintext_len
            + random_padding_len,
    );

    let (h, plaintext) = unsafe { buffer.split_at_mut_unchecked(24) };

    let msg_key = auth_key.compute_msg_key(plaintext, Side::Client);

    h[0..8].copy_from_slice(auth_key.id());
    h[8..24].copy_from_slice(&msg_key);

    let (aes_key, mut aes_iv) = auth_key.compute_aes_params(&msg_key, Side::Client);

    crate::crypto::aes_ige_encrypt(plaintext, &aes_key, &mut aes_iv);
}
