use crate::mtproto::{
    AuthKey, EncryptedHeader, EncryptedPadding, InternalHeader, Msg, PlainHeader, Side,
};

pub fn pack_plain(header: PlainHeader, buffer: &mut unbite::DynBuf, id: i64) {
    let mut header = header.into_buf();

    header.extend_from_slice(&0i64.to_le_bytes()); // auth_key_id
    header.extend_from_slice(&id.to_le_bytes()); // message_id
    header.extend_from_slice(&i32::try_from(buffer.len()).unwrap().to_le_bytes()); // message_data_length

    buffer.unsplit_buf_front(header);
}

pub fn pack_encrypted(
    header: EncryptedHeader,
    buffer: &mut unbite::DynBuf,
    padding: EncryptedPadding,
    auth_key: &AuthKey,
    internal: InternalHeader,
    msg: Msg,
) {
    let mut header = header.into_buf();

    let plaintext_len = buffer.len();

    // TODO: allow custom padding length; currently minimum possible.
    let random_padding_len = ((20 - (plaintext_len & 15)) & 15) + 12; // 12..28

    buffer.unsplit_raw_back(padding);

    buffer.init_with(|spare_capacity| {
        let dest = &mut spare_capacity[..random_padding_len];

        getrandom::fill_uninit(dest).unwrap()
    });

    header.extend_from_slice(&auth_key.id().get().to_le_bytes());

    // SAFETY: bytes in range 8..24 will be initialized with `msg_key`.
    unsafe { header.advance_unchecked(16) };

    header.extend_from_array(&internal.salt.to_le_bytes());
    header.extend_from_array(&internal.session_id.to_le_bytes());
    header.extend_from_array(&msg.msg_id.to_le_bytes());
    header.extend_from_array(&msg.seq_no.to_le_bytes());
    header.extend_from_array(&i32::try_from(plaintext_len).unwrap().to_le_bytes());

    buffer.unsplit_buf_front(header);

    let (h, plaintext) = buffer.as_mut_slice().split_at_mut(24);

    let msg_key = auth_key.compute_msg_key(plaintext, Side::Client);

    h[8..24].copy_from_slice(msg_key.as_ref());

    let (aes_key, mut aes_iv) = auth_key.compute_aes_params(&msg_key, Side::Client);

    crate::crypto::aes_ige_encrypt(plaintext, &aes_key, &mut aes_iv);
}
