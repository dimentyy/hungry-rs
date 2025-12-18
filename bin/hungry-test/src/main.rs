use bytes::BytesMut;

use hungry::tl::mtproto::enums::ServerDhParams;
use hungry::transport::{Packet, Unpack};
use hungry::{Envelope, mtproto, tl};
use hungry::writer::QueuedWriter;

const ADDR: &str = "149.154.167.40:443";

type ReaderDriver = tokio::net::tcp::OwnedReadHalf;
type WriterDriver = tokio::net::tcp::OwnedWriteHalf;

type Transport = hungry::transport::Full;

struct Plain<'a> {
    buffer: &'a mut BytesMut,
    reader: &'a mut hungry::reader::Reader<ReaderDriver, Transport>,
    writer: &'a mut hungry::writer::Writer<WriterDriver, Transport>,
}

impl<'a> Plain<'a> {
    async fn send<F: tl::Function>(
        &mut self,
        func: &F,
    ) -> Result<F::Response, hungry::plain::Error> {
        let transport_envelope = Envelope::split(self.buffer);
        let mtp_envelope = Envelope::split(self.buffer);

        let (_message_id, response) = hungry::plain::send(
            self.reader,
            self.writer,
            func,
            self.buffer,
            transport_envelope,
            mtp_envelope,
            0,
        )
        .await?;

        Ok(response)
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

    let buffer = BytesMut::with_capacity(1024 * 1024);

    let (mut reader, mut writer) = hungry::new(transport, r, buffer, w);

    let mut buffer = BytesMut::with_capacity(1024 * 1024);

    let mut plain = Plain {
        buffer: &mut buffer,
        reader: &mut reader,
        writer: &mut writer,
    };

    let mut nonce = tl::Int128::default();
    rand::fill(&mut nonce);

    let req_pq = hungry::auth::start(nonce);

    let func = req_pq.func();

    let tl::mtproto::enums::ResPq::ResPq(response) = dbg!(plain.send(func).await?);

    let res_pq = req_pq.res_pq(&response)?;

    let mut random_padding_bytes = [0; 192];
    rand::fill(&mut random_padding_bytes);

    let mut new_nonce = tl::Int256::default();
    rand::fill(&mut new_nonce);

    let mut req_dh_params = res_pq.req_dh_params(random_padding_bytes, new_nonce, &key);

    let mut temp_key = [0; 32];
    let mut key_aes_encrypted = [0; 256];

    let key_aes_encrypted = loop {
        rand::fill(&mut temp_key);

        if let Some(key_aes_encrypted) =
            req_dh_params.key_aes_encrypted(&temp_key, &mut key_aes_encrypted)
        {
            break key_aes_encrypted;
        }
    };

    let func = req_dh_params.func(key_aes_encrypted);

    let response = plain.send(func).await?;

    let response = match response {
        ServerDhParams::ServerDhParamsFail(_) => todo!(),
        ServerDhParams::ServerDhParamsOk(response) => response,
    };

    let server_dh_params_ok = req_dh_params.server_dh_params_ok(&response)?;

    let mut b = [0; 256];
    rand::fill(&mut b);

    let set_client_dh_params = server_dh_params_ok.set_client_dh_params(&b, 0);

    let func = set_client_dh_params.func();

    let response = plain.send(func).await?;

    let dh_gen_ok = {
        use tl::mtproto::enums::SetClientDhParamsAnswer::*;

        match response {
            DhGenOk(x) => x,
            DhGenRetry(_) => todo!(),
            DhGenFail(_) => todo!(),
        }
    };

    let (auth_key, salt) = set_client_dh_params.dh_gen_ok(dh_gen_ok)?;
    let session_id = rand::random();

    let mut sender = hungry::Sender::new(
        reader,
        QueuedWriter::new(writer),

        auth_key,

        salt,
        session_id
    );

    let func = tl::api::funcs::InvokeWithLayer {
        layer: 214,
        query: tl::api::funcs::InitConnection {
            api_id: 1,
            device_model: "device_model".to_string(),
            system_version: "system_version".to_string(),
            app_version: "0.0.0".to_string(),
            system_lang_code: "en".to_string(),
            lang_pack: "".to_string(),
            lang_code: "en".to_string(),
            proxy: None,
            params: None,
            query: tl::api::funcs::help::GetNearestDc {},
        }
    };

    dbg!(sender.invoke(&func));

    dbg!(sender.await?);

    Ok(())
}

fn since_epoch() -> std::time::Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
}
