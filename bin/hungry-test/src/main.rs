use bytes::BytesMut;

use hungry::{tl, Envelope};
use hungry::reader::{Dump, Split};

const ADDR: &str = "149.154.167.40:443";

type Transport = hungry::transport::Full;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let stream = tokio::net::TcpStream::connect(ADDR).await?;
    let (r, w) = stream.into_split();

    let behaviour = Dump(Split);

    let buffer = BytesMut::with_capacity(1024);

    let (mut reader, mut writer) = hungry::new::<Transport, _, _, _>(r, behaviour, buffer, w);

    let mut buffer = BytesMut::with_capacity(1024);

    let transport_envelope = Envelope::split(&mut buffer);
    let mtp_envelope = Envelope::split(&mut buffer);

    let mut nonce = tl::Int128::default();
    rand::fill(&mut nonce);

    let func = tl::mtproto::funcs::ReqPqMulti { nonce };

    let response = hungry::plain::send(
        &mut reader,
        &mut writer,
        &func,
        &mut buffer,
        transport_envelope,
        mtp_envelope,
        0
    ).await?;

    dbg!(&response);

    Ok(())
}
