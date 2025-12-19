use std::future::poll_fn;
use std::pin::pin;
use std::task::Poll;
use bytes::BytesMut;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use hungry::mtproto::{AuthKey, Salt};
use hungry::reader::Reader;
use hungry::tl::mtproto::enums::ServerDhParams;
use hungry::writer::{QueuedWriter, Writer};
use hungry::{Envelope, tl};
use hungry::transport::Full;

const ADDR: &str = "149.154.167.40:443";

const N: &str = "253428894488404155649716895907134732068988477590847790525820265945460224638539\
    4058588521595116849196570822264939918060381807420062046377613542488463216251240316379308392\
    1641631564740959529419359595852941166848940585952337613333022396096584117954892216031229237\
    3029437018775884567383353986024616752250817918203931537575049526362349513232378200365435810\
    4782690612092797248736680529211579223142368426126233039432475078545094258975175539015664775\
    1460719351439969059949569615302809050721500330239005077889855323917509948255722081644689442\
    127297605422579707142646660768825302832201908302295573257427896031830742328565032949";

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

async fn generate_auth_key(
    public_key: hungry::crypto::RsaKey,
    reader: &mut Reader<ReaderDriver, Transport>,
    writer: &mut Writer<WriterDriver, Transport>,
) -> anyhow::Result<(AuthKey, Salt)> {
    let mut buffer = BytesMut::with_capacity(1024 * 1024);

    let mut plain = Plain {
        buffer: &mut buffer,
        reader,
        writer,
    };

    let mut nonce = tl::Int128::default();
    rand::fill(&mut nonce);

    let req_pq = hungry::auth::start(nonce);

    let func = req_pq.func();

    let tl::mtproto::enums::ResPq::ResPq(response) = plain.send(func).await?;

    println!("ResPq");

    let res_pq = req_pq.res_pq(&response)?;

    let mut random_padding_bytes = [0; 192];
    rand::fill(&mut random_padding_bytes);

    let mut new_nonce = tl::Int256::default();
    rand::fill(&mut new_nonce);

    let mut req_dh_params = res_pq.req_dh_params(random_padding_bytes, new_nonce, &public_key);

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

    println!("ServerDhParamsOk");

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

    println!("DhGenOk");

    Ok((auth_key, salt))
}

async fn async_main() -> anyhow::Result<()> {
    let n = hungry::rug::Integer::from_str_radix(N, 10)?;
    let e = hungry::rug::Integer::from(65537);

    let public_key = hungry::crypto::RsaKey::new(n, e); // fingerprint: -5595554452916591101

    let transport = Transport::default();

    let (r, w) = tokio::net::TcpStream::connect(ADDR).await?.into_split();

    let buffer = BytesMut::with_capacity(1024 * 1024);

    let (mut reader, mut writer) = hungry::init(transport, r, buffer, w);

    let (auth_key, salt) = generate_auth_key(public_key, &mut reader, &mut writer).await?;

    let session_id = rand::random();

    let mut sender = hungry::Sender::new(
        reader,
        QueuedWriter::new(writer),
        auth_key,
        salt,
        session_id,
    );

    let func = tl::mtproto::funcs::Ping { ping_id: 123 };
    dbg!(sender.invoke(&func));

    let func = tl::api::funcs::InvokeWithLayer {
        layer: 214,
        query: tl::api::funcs::InitConnection {
            api_id: 4,
            device_model: "device_model".to_string(),
            system_version: "system_version".to_string(),
            app_version: "0.0.0".to_string(),
            system_lang_code: "en".to_string(),
            lang_pack: "".to_string(),
            lang_code: "en".to_string(),
            proxy: None,
            params: None,
            query: tl::api::funcs::help::GetNearestDc {},
        },
    };
    dbg!(sender.invoke(&func));

    loop {
        dbg!(poll_fn(|cx| {
            let mut result = match sender.poll(cx) {
                Poll::Ready(Ok(result)) => result,
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => return Poll::Pending,
            };

            // for result in result {
            //     match result {
            //         Ok(_) => {}
            //         Err(err) => todo!(),
            //     }
            // }

            result.next().unwrap().unwrap();

            cx.waker().wake_by_ref();

            Poll::Pending
        }).await)?;
    }

    Ok(())
}
