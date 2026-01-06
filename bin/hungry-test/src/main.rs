use std::future::poll_fn;

use tokio::io::AsyncWriteExt;

use hungry::reader::ReaderResult;
use hungry::transport::{Transport as _, Unpack};
use hungry::{crypto_bigint, mtproto, tl, unbite};

use tl::Identifiable;

use crypto_bigint::{Odd, U2048};

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

    let r_buffer = unbite::DynBuf::new(1024 * 1024);
    let w_buffer = unbite::DynBuf::new(1024 * 1024);

    let (mut r, mut w) = hungry::init(transport, r, r_buffer, w, w_buffer);

    let hungry::writer::OwnedWriteInner {
        driver: mut w,
        mut buffer,
    } = poll_fn(|cx| w.poll(cx)).await?;

    let envelope = Transport::envelope(&mut buffer);

    let header = buffer.split_raw_front();

    let mut nonce = tl::Int128::default();

    getrandom::fill(nonce.as_mut())?;

    let req_pq_multi = hungry::auth::start(nonce);

    buffer.init_with(|spare_capacity| {
        let mut buf = tl::ser::Buf::uninit(spare_capacity);

        buf.ser(&tl::mtproto::funcs::ReqPqMulti::CONSTRUCTOR_ID);
        buf.ser(req_pq_multi.func());

        buf.as_slice()
    });

    let mut fut = w.single_plain(envelope, header, &mut buffer, 0);

    poll_fn(|cx| fut.poll(cx)).await?;

    let unpack = match poll_fn(|cx| r.poll(cx)).await {
        ReaderResult::Reserve(_) => todo!(),
        ReaderResult::Unpack(unpack) => unpack,
        ReaderResult::Error(err) => return Err(err.into()),
    };

    let data = match unpack {
        Unpack::Packet(packet) => packet.data,
        Unpack::QuickAck(_) => todo!(),
    };

    let _ = match mtproto::Message::unpack(&r.buffer().as_slice()[data.clone()]) {
        mtproto::Message::Plain(message) => message,
        mtproto::Message::Encrypted(_) => todo!(),
    };

    let data = data.start + mtproto::PlainMessage::HEADER_LEN..data.end;

    let mut buf = tl::de::Buf::new(&r.buffer().as_slice()[data]);

    let tl::mtproto::enums::ResPq::ResPq(res_pq) = buf.de()?;

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

    buffer.clear();

    let envelope = Transport::envelope(&mut buffer);

    let header = buffer.split_raw_front();

    buffer.init_with(|spare_capacity| {
        let mut buf = tl::ser::Buf::uninit(spare_capacity);

        buf.ser(&tl::mtproto::funcs::ReqDhParams::CONSTRUCTOR_ID);
        buf.ser(dbg!(func));

        buf.as_slice()
    });

    let mut fut = w.single_plain(envelope, header, &mut buffer, 0);

    poll_fn(|cx| fut.poll(cx)).await?;

    let unpack = match poll_fn(|cx| r.poll(cx)).await {
        ReaderResult::Reserve(_) => todo!(),
        ReaderResult::Unpack(unpack) => unpack,
        ReaderResult::Error(err) => return Err(err.into()),
    };

    let data = match unpack {
        Unpack::Packet(packet) => packet.data,
        Unpack::QuickAck(_) => todo!(),
    };

    let _ = match mtproto::Message::unpack(&r.buffer().as_slice()[data.clone()]) {
        mtproto::Message::Plain(message) => message,
        mtproto::Message::Encrypted(_) => todo!(),
    };

    let data = data.start + mtproto::PlainMessage::HEADER_LEN..data.end;

    let mut buf = tl::de::Buf::new(&r.buffer().as_slice()[data]);

    let _: tl::mtproto::enums::ServerDhParams = dbg!(buf.de()?);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main())
}
