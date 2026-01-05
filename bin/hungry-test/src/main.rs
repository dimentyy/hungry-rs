use std::future::poll_fn;

use tokio::io::AsyncWriteExt;

use hungry::tl::ser::SerializeUnchecked;
use hungry::tl::{ConstSerializedLen, Identifiable};
use hungry::transport::{Transport as _, TransportInit, Unpack};
use hungry::unbite;

const ADDR: &str = "149.154.167.40:443";

type Transport = hungry::transport::Full;

async fn async_main() -> anyhow::Result<()> {
    let transport = Transport::default();

    let (r, w) = tokio::net::TcpStream::connect(ADDR).await?.into_split();

    let r_buffer = unbite::DynBuf::new(1024 * 1024);

    let (mut r, init, mut w) = hungry::init(transport, r, r_buffer, w);

    let mut buffer = unbite::DynBuf::new(1024 * 1024);
    init.init(&mut w.transport, &mut buffer);

    w.driver().write(buffer.as_slice()).await?;

    buffer.clear();

    let envelope = Transport::envelope(&mut buffer);

    let header = buffer.split_raw_front();

    let mut nonce = hungry::tl::Int128::default();

    getrandom::fill(nonce.as_mut())?;

    let func = dbg!(hungry::tl::mtproto::funcs::ReqPqMulti { nonce });

    unsafe {
        let mut buf = buffer.as_non_null();

        buf = hungry::tl::mtproto::funcs::ReqPqMulti::CONSTRUCTOR_ID.serialize_unchecked(buf);
        func.serialize_unchecked(buf);

        buffer.set_len(4 + hungry::tl::mtproto::funcs::ReqPqMulti::SERIALIZED_LEN);
    }

    let mut fut = w.single_plain(envelope, header, &mut buffer, 0);

    poll_fn(|cx| fut.poll(cx)).await?;

    let unpack = dbg!(poll_fn(|cx| r.poll(cx)).await?);

    let data = match unpack {
        Unpack::Packet(packet) => packet.data,
        Unpack::QuickAck(_) => todo!(),
    };

    let _ = match dbg!(hungry::mtproto::Message::unpack(&r.buffer().as_slice()[data.clone()])) {
        hungry::mtproto::Message::Plain(message) => message,
        hungry::mtproto::Message::Encrypted(_) => todo!(),
    };

    let data = data.start + hungry::mtproto::PlainMessage::HEADER_LEN..data.end;

    let mut buf = hungry::tl::de::Buf::new(&r.buffer().as_slice()[data]);

    let _: hungry::tl::mtproto::enums::ResPq = dbg!(buf.de()?);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async_main())
}
