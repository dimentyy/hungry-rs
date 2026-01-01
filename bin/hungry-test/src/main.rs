use std::time::Instant;

use grammers_tl_types::{Deserializable, Serializable};

use hungry::tl::api::{enums, types};
use hungry::tl::de::Deserialize;
use hungry::tl::ser::SerializeInto;

fn main() {
    let hungry_message: enums::Message = types::Message {
        out: false,
        mentioned: true,
        media_unread: false,
        silent: true,
        post: false,
        from_scheduled: false,
        legacy: true,
        edit_hide: false,
        pinned: false,
        noforwards: true,
        invert_media: true,
        offline: true,
        video_processing_pending: false,
        paid_suggested_post_stars: false,
        paid_suggested_post_ton: true,
        id: 128936124,
        from_id: Some(
            types::PeerUser {
                user_id: 892479289174,
            }
            .into(),
        ),
        from_boosts_applied: Some(12944213),
        peer_id: types::PeerChat { chat_id: 9812497 }.into(),
        saved_peer_id: Some(
            types::PeerChannel {
                channel_id: 2189317320138,
            }
            .into(),
        ),
        fwd_from: None,
        via_bot_id: None,
        via_business_bot_id: None,
        reply_to: None,
        date: 337648364,
        message:
            "dodgags8ouido8g71b2ued od8asou8dgwdqwq .  e2839peqha *(dphip8s dad 72378qas dsauhiu"
                .to_owned(),
        media: Some(
            types::MessageMediaGiveaway {
                only_new_subscribers: false,
                winners_are_visible: false,
                channels: vec![
                    12041608723,
                    4096030353,
                    12892387213293,
                    21312783293,
                    2198331783892,
                ],
                countries_iso_2: Some(vec!["ru".to_owned(), "en".to_owned(), "ch".to_owned()]),
                prize_description: Some("snasauis89y 1282282u ajskasd".to_owned()),
                quantity: 9834143,
                months: Some(1293829),
                stars: Some(21893127932),
                until_date: 198223813,
            }
            .into(),
        ),
        reply_markup: None,
        entities: None,
        views: None,
        forwards: None,
        replies: None,
        edit_date: None,
        post_author: None,
        grouped_id: None,
        reactions: None,
        restriction_reason: None,
        ttl_period: Some(938932392),
        quick_reply_shortcut_id: None,
        effect: None,
        factcheck: None,
        report_delivery_until_date: None,
        paid_message_stars: Some(219839837238921783),
        suggested_post: None,
    }
    .into();

    let grammers_message: grammers_tl_types::enums::Message = grammers_tl_types::types::Message {
        out: false,
        mentioned: true,
        media_unread: false,
        silent: true,
        post: false,
        from_scheduled: false,
        legacy: true,
        edit_hide: false,
        pinned: false,
        noforwards: true,
        invert_media: true,
        offline: true,
        video_processing_pending: false,
        paid_suggested_post_stars: false,
        paid_suggested_post_ton: true,
        id: 128936124,
        from_id: Some(
            grammers_tl_types::types::PeerUser {
                user_id: 892479289174,
            }
            .into(),
        ),
        from_boosts_applied: Some(12944213),
        peer_id: grammers_tl_types::types::PeerChat { chat_id: 9812497 }.into(),
        saved_peer_id: Some(
            grammers_tl_types::types::PeerChannel {
                channel_id: 2189317320138,
            }
            .into(),
        ),
        fwd_from: None,
        via_bot_id: None,
        via_business_bot_id: None,
        reply_to: None,
        date: 337648364,
        message:
            "dodgags8ouido8g71b2ued od8asou8dgwdqwq .  e2839peqha *(dphip8s dad 72378qas dsauhiu"
                .to_owned(),
        media: Some(
            grammers_tl_types::types::MessageMediaGiveaway {
                only_new_subscribers: false,
                winners_are_visible: false,
                channels: vec![
                    12041608723,
                    4096030353,
                    12892387213293,
                    21312783293,
                    2198331783892,
                ],
                countries_iso2: Some(vec!["ru".to_owned(), "en".to_owned(), "ch".to_owned()]),
                prize_description: Some("snasauis89y 1282282u ajskasd".to_owned()),
                quantity: 9834143,
                months: Some(1293829),
                stars: Some(21893127932),
                until_date: 198223813,
            }
            .into(),
        ),
        reply_markup: None,
        entities: None,
        views: None,
        forwards: None,
        replies: None,
        edit_date: None,
        post_author: None,
        grouped_id: None,
        reactions: None,
        restriction_reason: None,
        ttl_period: Some(938932392),
        quick_reply_shortcut_id: None,
        effect: None,
        factcheck: None,
        report_delivery_until_date: None,
        paid_message_stars: Some(219839837238921783),
        suggested_post: None,
    }
    .into();

    let mut buf = Vec::with_capacity(10_000_000);

    let t1 = Instant::now();

    for _ in 0..10_000 {
        buf.clear();

        for _ in 0..10_000 {
            grammers_message.serialize(&mut buf);
        }
    }

    let t2 = Instant::now();
    dbg!(t2 - t1);

    for _ in 0..10_000 {
        buf.clear();

        for _ in 0..10_000 {
            buf.ser(&hungry_message);
        }
    }

    let t3 = Instant::now();
    dbg!(t3 - t2);

    for _ in 0..10_000 {
        let mut buf = hungry::tl::de::Buf::new(&buf);

        for _ in 0..10_000 {
            hungry::tl::api::enums::Message::deserialize(&mut buf).unwrap();
        }
    }

    let t4 = Instant::now();
    dbg!(t4 - t3);

    for _ in 0..10_000 {
        let mut buf = grammers_tl_types::deserialize::Cursor::from_slice(&buf);

        for _ in 0..10_000 {
            grammers_tl_types::enums::Message::deserialize(&mut buf).unwrap();
        }
    }

    let t5 = Instant::now();
    dbg!(t5 - t4);

    for _ in 0..10_000 {
        buf.clear();
        buf.push(0);

        for _ in 0..10_000 {
            grammers_message.serialize(&mut buf);
        }
    }

    let t6 = Instant::now();
    dbg!(t6 - t5);

    for _ in 0..10_000 {
        let mut buf = grammers_tl_types::deserialize::Cursor::from_slice(&buf[1..]);

        for _ in 0..10_000 {
            grammers_tl_types::enums::Message::deserialize(&mut buf).unwrap();
        }
    }

    let t7 = Instant::now();
    dbg!(t7 - t6);
}
