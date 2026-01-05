use std::future::poll_fn;

use tokio::io::AsyncWriteExt;

use hungry::reader::ReaderResult;
use hungry::transport::{Transport as _, TransportInit, Unpack};
use hungry::{mtproto, tl, unbite};

use tl::ser::SerializeUnchecked;
use tl::{ConstSerializedLen, Identifiable};

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

    let mut nonce = tl::Int128::default();

    getrandom::fill(nonce.as_mut())?;

    let func = dbg!(tl::mtproto::funcs::ReqPqMulti { nonce });

    unsafe {
        let mut buf = buffer.as_non_null();

        buf = tl::mtproto::funcs::ReqPqMulti::CONSTRUCTOR_ID.serialize_unchecked(buf);
        func.serialize_unchecked(buf);

        buffer.set_len(4 + tl::mtproto::funcs::ReqPqMulti::SERIALIZED_LEN);
    }

    let mut fut = w.single_plain(envelope, header, &mut buffer, 0);

    poll_fn(|cx| fut.poll(cx)).await?;

    let unpack = match dbg!(poll_fn(|cx| r.poll(cx)).await) {
        ReaderResult::Reserve { .. } => todo!(),
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
