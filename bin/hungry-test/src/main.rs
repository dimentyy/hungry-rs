use std::future::poll_fn;

use tokio::io::AsyncWriteExt;

use hungry::reader::ReaderResult;
use hungry::transport::{Transport as _, Unpack};
use hungry::{mtproto, tl, unbite};

use tl::Identifiable;

const ADDR: &str = "149.154.167.40:443";

type Transport = hungry::transport::Full;

async fn async_main() -> anyhow::Result<()> {
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

    let func = dbg!(tl::mtproto::funcs::ReqPqMulti { nonce });

    buffer.init_with(|spare_capacity| {
        let mut buf = tl::ser::Buf::uninit(spare_capacity);

        buf.ser(&tl::mtproto::funcs::ReqPqMulti::CONSTRUCTOR_ID);
        buf.ser(&func);

        buf.as_slice()
    });

    let mut fut = w.single_plain(envelope, header, &mut buffer, 0);

    poll_fn(|cx| fut.poll(cx)).await?;

    let unpack = match dbg!(poll_fn(|cx| r.poll(cx)).await) {
        ReaderResult::Reserve(_) => todo!(),
        ReaderResult::Unpack(unpack) => unpack,
        ReaderResult::Error(err) => return Err(err.into()),
    };

    let data = match unpack {
        Unpack::Packet(packet) => packet.data,
        Unpack::QuickAck(_) => todo!(),
    };

    let _ = match dbg!(mtproto::Message::unpack(
        &r.buffer().as_slice()[data.clone()]
    )) {
        mtproto::Message::Plain(message) => message,
        mtproto::Message::Encrypted(_) => todo!(),
    };

    let data = data.start + mtproto::PlainMessage::HEADER_LEN..data.end;

    let mut buf = tl::de::Buf::new(&r.buffer().as_slice()[data]);

    let _: tl::mtproto::enums::ResPq = dbg!(buf.de()?);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main())
}
