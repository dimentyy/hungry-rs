use std::future::poll_fn;
use std::task::{Poll, ready};

use hungry::{crypto_bigint, mtproto, tl, unbite};

use crypto_bigint::{Odd, U2048};

use tl::mtproto::{enums, funcs, types};
use tl::{Identifiable, SerializedLen};

const ADDR: &str = "149.154.167.40:443";

const N: &str = "253428894488404155649716895907134732068988477590847790525820265945460224638539\
    4058588521595116849196570822264939918060381807420062046377613542488463216251240316379308392\
    1641631564740959529419359595852941166848940585952337613333022396096584117954892216031229237\
    3029437018775884567383353986024616752250817918203931537575049526362349513232378200365435810\
    4782690612092797248736680529211579223142368426126233039432475078545094258975175539015664775\
    1460719351439969059949569615302809050721500330239005077889855323917509948255722081644689442\
    127297605422579707142646660768825302832201908302295573257427896031830742328565032949";

type Transport = hungry::transport::Full;

async fn async_main() -> anyhow::Result<()> {
    let n = Odd::new(U2048::from_str_radix_vartime(N, 10)?).unwrap();
    let e = Odd::new(U2048::from_word(65537)).unwrap();

    let server_public_key = hungry::crypto::RsaKey::new(n, e); // fingerprint: -5595554452916591101

    let transport = Transport::default();

    let (r, w) = tokio::net::TcpStream::connect(ADDR).await?.into_split();

    let r_buffer = unbite::DynBuf::new(16 * 1024);
    let w_buffer = unbite::DynBuf::new(16 * 1024);

    let (r, w) = hungry::init(transport, r, r_buffer, w, w_buffer);

    let hungry::writer::OwnedWriteInner {
        driver: w,
        mut buffer,
    } = w.await?;

    let mut nonce = tl::Int128::default();

    getrandom::fill(nonce.as_mut())?;

    let req_pq_multi = hungry::auth::start(nonce);

    let mut plain = hungry::plain::Plain::new(r, w);

    let enums::ResPq::ResPq(res_pq) = plain.send(&mut buffer, req_pq_multi.func()).await?;

    let res_pq = req_pq_multi.res_pq(&res_pq)?;

    let mut random_padding_bytes = [0; 192];
    getrandom::fill(&mut random_padding_bytes)?;

    let mut new_nonce = tl::Int256::default();
    getrandom::fill(new_nonce.as_mut())?;

    let mut req_dh_params =
        res_pq.req_dh_params(random_padding_bytes, new_nonce, &server_public_key);

    let mut temp_key = [0; 32];

    let func = loop {
        getrandom::fill(&mut temp_key)?;

        if let Some(func) = req_dh_params.func(&temp_key) {
            break func;
        }
    };

    let enums::ServerDhParams::ServerDhParamsOk(server_dh_params) =
        plain.send(&mut buffer, func).await?
    else {
        todo!()
    };

    let server_dh_params = req_dh_params.server_dh_params_ok(&server_dh_params)?;

    let mut b = [0; 256];
    getrandom::fill(&mut b)?;

    let set_client_dh_params =
        server_dh_params.set_client_dh_params(U2048::from_be_slice(&b), 0)?;

    let enums::SetClientDhParamsAnswer::DhGenOk(dh_gen_ok) =
        plain.send(&mut buffer, set_client_dh_params.func()).await?
    else {
        todo!()
    };

    let hungry::auth::DhGenOk {
        auth_key,
        server_salt,
    } = dbg!(set_client_dh_params.dh_gen_ok(&dh_gen_ok)?);

    let (r, w) = plain.into_inner();

    let w = hungry::writer::QueuedWriter::new(w);

    let session = getrandom::u64()? as i64;

    let sender = hungry::sender::Sender::new(r, w, auth_key, session, server_salt);

    let mut handle = hungry::handle::Handle::new(sender);

    let func = tl::ConstructorId(tl::api::funcs::InvokeWithLayer {
        layer: 214,
        query: tl::api::funcs::InitConnection {
            api_id: 1,
            device_model: "device_model".to_string(),
            system_version: "system_version".to_string(),
            app_version: "0.0.1".to_string(),
            system_lang_code: "en".to_string(),
            lang_pack: "".to_string(),
            lang_code: "en".to_string(),
            proxy: None,
            params: None,
            query: tl::api::funcs::help::GetNearestDc {},
        },
    });
    let get_nearest_dc_rx = handle.invoke(func.serialized_len(), |buf| buf.ser(&func));

    let task = tokio::spawn(async move {
        loop {
            let _objects = poll_fn(|cx| dbg!(handle.poll(cx))).await.unwrap();
        }
    });

    let tl::Object::api_NearestDc(tl::api::enums::NearestDc::NearestDc(nearest_dc)) =
        get_nearest_dc_rx.await?
    else {
        todo!("welp...")
    };

    dbg!(nearest_dc);

    task.await?
}

fn main() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}
