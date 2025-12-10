use bytes::BytesMut;
use std::time::{SystemTime, UNIX_EPOCH};

use hungry::mtproto::AuthKey;
use hungry::reader::{Dump, Parted, Reserve, Split};
use hungry::tl::mtproto::enums::SetClientDhParamsAnswer;
use hungry::tl::ser::Serialize;
use hungry::{Envelope, mtproto, tl};

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
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

    let response = dbg!(plain.send(func).await?);

    let res_pq = req_pq.res_pq(response);

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

    let server_time = server_dh_params.server_time();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before epoch")
        .as_secs() as i32;

    dbg!(now - server_time);

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

    let transport = Envelope::split(&mut buffer);
    let mtp = Envelope::split(&mut buffer);

    // let func = tl::api::funcs::InvokeWithLayer {
    //     layer: 218,
    //     query: tl::api::funcs::InitConnection {
    //         api_id: 1,
    //         device_model: "MacOS 64-bit".to_string(),
    //         system_version: "26.0.1".to_string(),
    //         app_version: "0.8.1".to_string(),
    //         system_lang_code: "en".to_string(),
    //         lang_pack: "".to_string(),
    //         lang_code: "en".to_string(),
    //         proxy: None,
    //         params: None,
    //         query: tl::api::funcs::help::GetConfig {},
    //     },
    // };

    let func = tl::mtproto::funcs::GetFutureSalts { num: 64 };

    let msg_id = get_new_msg_id();

    msg_id.serialize_into(&mut buffer);

    1i32.serialize_into(&mut buffer);

    (func.serialized_len() as i32).serialize_into(&mut buffer);

    func.serialize_into(&mut buffer);

    let session_id = rand::random();

    let message = mtproto::DecryptedMessage { salt, session_id };

    writer
        .single(&mut buffer, transport, mtp, &auth_key, &message)
        .await?;

    loop {
        dbg!((&mut reader).await?);
    }
}

fn get_new_msg_id() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before epoch");

    let seconds = (now.as_secs() as i32) as u64;
    let nanoseconds = 0; //now.subsec_nanos() as u64;
    let new_msg_id = ((seconds >> 5) << 37) as i64;

    new_msg_id
}
