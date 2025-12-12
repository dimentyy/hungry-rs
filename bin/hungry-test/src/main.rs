use bytes::BytesMut;

use hungry::reader::{Dump, Parted, Reserve, Split};
use hungry::transport::{Packet, Unpack};
use hungry::{Envelope, mtproto, tl};

use tl::de::Deserialize;
use tl::mtproto::enums::SetClientDhParamsAnswer;
use tl::ser::Serialize;

const ADDR: &str = "149.154.167.40:443";

type ReaderDriver = tokio::net::tcp::OwnedReadHalf;
type WriterDriver = tokio::net::tcp::OwnedWriteHalf;

type Transport = hungry::transport::Full;

type ReaderHandle = Dump<Parted<Reserve, Split>>;

struct Plain<'a> {
    buffer: &'a mut BytesMut,
    reader: &'a mut hungry::reader::Reader<ReaderDriver, ReaderHandle, Transport>,
    writer: &'a mut hungry::writer::Writer<WriterDriver, Transport>,
}

impl<'a> Plain<'a> {
    fn send<F: tl::Function>(
        &mut self,
        func: &F,
    ) -> impl Future<Output = Result<F::Response, hungry::plain::Error>> {
        let transport_envelope = Envelope::split(&mut self.buffer);
        let mtp_envelope = Envelope::split(&mut self.buffer);

        hungry::plain::send(
            self.reader,
            self.writer,
            func,
            self.buffer,
            transport_envelope,
            mtp_envelope,
            0,
        )
    }
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    const N: &str = "25342889448840415564971689590713473206898847759084779052582026594546022463853\
        940585885215951168491965708222649399180603818074200620463776135424884632162512403163793083\
        921641631564740959529419359595852941166848940585952337613333022396096584117954892216031229\
        237302943701877588456738335398602461675225081791820393153757504952636234951323237820036543\
        581047826906120927972487366805292115792231423684261262330394324750785450942589751755390156\
        647751460719351439969059949569615302809050721500330239005077889855323917509948255722081644\
        689442127297605422579707142646660768825302832201908302295573257427896031830742328565032949";

    let n = hungry::rug::Integer::from_str_radix(N, 10)?;
    let e = hungry::rug::Integer::from(65537);

    let key = hungry::crypto::RsaKey::new(n, e); // fingerprint: -5595554452916591101

    let transport = Transport::default();

    let stream = tokio::net::TcpStream::connect(ADDR).await?;
    let (r, w) = stream.into_split();

    let handle = Dump(Parted {
        buffer: Reserve,
        output: Split,
    });

    let buffer = BytesMut::with_capacity(1024 * 1024);

    let (mut reader, mut writer) = hungry::new(transport, r, handle, buffer, w);

    let mut buffer = BytesMut::with_capacity(1024 * 1024);

    let mut plain = Plain {
        buffer: &mut buffer,
        reader: &mut reader,
        writer: &mut writer,
    };

    let mut nonce = tl::Int128::default();
    rand::fill(&mut nonce);

    let req_pq = hungry::auth::start(nonce);

    let func = dbg!(req_pq.func());

    let tl::mtproto::enums::ResPq::ResPq(response) = dbg!(plain.send(func).await?);

    let res_pq = req_pq.res_pq(&response)?;

    let mut random_padding_bytes = [0; 192];
    rand::fill(&mut random_padding_bytes);

    let mut new_nonce = tl::Int256::default();
    rand::fill(&mut new_nonce);

    let mut req_dh_params = res_pq.req_dh_params(random_padding_bytes, new_nonce, &key);

    let mut temp_key = [0; 32];
    let mut key_aes_encrypted = [0; 256];

    loop {
        rand::fill(&mut temp_key);

        if req_dh_params.key_aes_encrypted(&temp_key, &mut key_aes_encrypted) {
            break;
        }
    }

    let func = dbg!(req_dh_params.func(&key_aes_encrypted));

    let response = dbg!(plain.send(func).await?);

    let server_dh_params = req_dh_params.server_dh_params(response);

    let mut b = [0; 256];
    rand::fill(&mut b);

    let set_client_dh_params = server_dh_params.set_client_dh_params(&b, 0);

    let func = dbg!(set_client_dh_params.func());

    let response = dbg!(plain.send(func).await?);

    let dh_gen_ok = match response {
        SetClientDhParamsAnswer::DhGenOk(x) => x,
        SetClientDhParamsAnswer::DhGenRetry(_) => todo!(),
        SetClientDhParamsAnswer::DhGenFail(_) => todo!(),
    };

    let (auth_key, salt) = set_client_dh_params.dh_gen_ok(dh_gen_ok);
    let session_id = rand::random();

    let transport = Envelope::split(&mut buffer);
    let mtp = Envelope::split(&mut buffer);

    let mut msg_ids = mtproto::MsgIds::new();
    let mut seq_nos = mtproto::SeqNos::new();

    let func = tl::mtproto::funcs::GetFutureSalts { num: 1 };

    let message = mtproto::Msg {
        msg_id: msg_ids.get(since_epoch()),
        seq_no: seq_nos.get_content_related(),
    };

    let mut msg_container = hungry::MsgContainer::new(buffer);

    msg_container.push(message, &func).unwrap();

    let mut buffer = msg_container.finalize();

    let message = mtproto::DecryptedMessage { salt, session_id };

    let msg = mtproto::Msg {
        msg_id: msg_ids.get(since_epoch()),
        seq_no: seq_nos.get_content_related(),
    };

    writer
        .single(&mut buffer, transport, mtp, &auth_key, message, msg)
        .await?;

    loop {
        let (mut buffer, unpack) = (&mut reader).await?;

        let (data, next) = match unpack {
            Unpack::Packet(Packet { data, next }) => (data, next),
            Unpack::QuickAck(_) => todo!(),
        };

        let encrypted = match mtproto::Message::unpack(&buffer[data.clone()]) {
            mtproto::Message::Plain(_) => todo!(),
            mtproto::Message::Encrypted(message) => message,
        };

        assert_eq!(&encrypted.auth_key_id.get().to_le_bytes(), auth_key.id());

        let decrypted = encrypted.decrypt(
            &auth_key,
            &mut buffer[data.start + mtproto::EncryptedMessage::HEADER_LEN..data.end],
        );

        assert_eq!(decrypted.salt, salt);
        assert_eq!(decrypted.session_id, session_id);

        let buffer = &buffer[data.start
            + mtproto::EncryptedMessage::HEADER_LEN
            + mtproto::DecryptedMessage::HEADER_LEN..data.end];

        let mut buf = tl::de::Buf::new(buffer);

        let _message: mtproto::Msg = buf.infallible();
        let bytes = i32::deserialize_checked(&mut buf)? as usize;

        assert!(buffer.len() - 20 >= bytes);

        let id = u32::deserialize_checked(&mut buf)?;

        match id {
            0x73f1f8dc => {}
            0xf35c6d01 => {
                println!("rpc result, todo");
                continue;
            }
            _ => todo!(),
        }

        let container = mtproto::MsgContainer::new(&mut buf)?;

        for message in container {
            let (message, mut buf) = message?;

            let id = u32::deserialize_checked(&mut buf)?;

            match id {
                0x9ec20908 => {
                    let session =
                        tl::mtproto::types::NewSessionCreated::deserialize_checked(&mut buf)?;

                    dbg!(session);
                }
                0xae500895 => {
                    let salts = tl::mtproto::types::FutureSalts::deserialize_checked(&mut buf)?;

                    dbg!(salts);
                }
                0x62d6b459 => {
                    let ack = tl::mtproto::types::MsgsAck::deserialize_checked(&mut buf)?;

                    dbg!(ack);
                }
                _ => todo!(),
            }
        }
    }
}

fn since_epoch() -> std::time::Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
}
