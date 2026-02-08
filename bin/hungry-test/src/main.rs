mod auth;
mod client;
mod key;
mod updates;

use std::fmt;
use std::future::poll_fn;
use std::pin::pin;
use std::task::Poll;

use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;

use hungry::{tl, unbite};

use crate::auth::auth;
use crate::key::get_auth_key;
use crate::updates::process_updates;
pub use client::{Client, Request, RequestError};

const ADDR: &str = "149.154.167.40:443";

type R = tokio::net::tcp::OwnedReadHalf;
type W = tokio::net::tcp::OwnedWriteHalf;

type Transport = hungry::transport::Full;

type Plain = hungry::plain::Plain<Transport, R, W>;

type Sender = hungry::sender::Sender<Transport, R, W, Handle>;

struct Handle {}

impl hungry::sender::Handle for Handle {
    type Extra = oneshot::Sender<Result<tl::Object, RequestError>>;

    fn rpc_result(&mut self, extra: Self::Extra, typ: u32, buf: &mut tl::de::Buf<'_>) {
        let obj = tl::Object::deserialize(typ, buf).unwrap();
        extra.send(Ok(obj)).unwrap();
    }

    fn rpc_result_error(&mut self, extra: Self::Extra, error: tl::mtproto::types::RpcError) {
        extra.send(Err(RequestError::RpcError(error))).unwrap();
    }

    fn bad_server_salt(&mut self, extra: Self::Extra) {
        extra.send(Err(RequestError::BadServerSalt)).unwrap();
    }
}

async fn connect() -> anyhow::Result<(Plain, unbite::DynBuf)> {
    let transport = Transport::default();

    let (r, w) = tokio::net::TcpStream::connect(ADDR).await?.into_split();

    let r_buffer = unbite::DynBuf::new(64 * 1024);
    let w_buffer = unbite::DynBuf::new(64 * 1024);

    let (r, w) = hungry::init(transport, r, r_buffer, w, w_buffer);

    let hungry::writer::OwnedWriteInner { driver: w, buffer } = w.await?;

    let plain = Plain::new(r, w);

    Ok((plain, buffer))
}

fn spawn<E, F>(future: F) -> tokio::task::JoinHandle<Result<(), E>>
where
    E: fmt::Display + Send + 'static,
    F: Future<Output = Result<(), E>> + Send + 'static,
{
    tokio::spawn(async move { future.await.inspect_err(|err| error!(%err)) })
}

async fn async_main() -> anyhow::Result<()> {
    let api_id = std::env::var("API_ID")?.parse()?;
    let api_hash = std::env::var("API_HASH")?;
    let bot_auth_token = std::env::var("BOT_AUTH_TOKEN")?;

    let (mut plain, mut buffer) = connect().await?;

    let (auth_key, server_salt) = get_auth_key(&mut plain, &mut buffer).await?;

    let (r, w) = plain.into_inner();

    let w = hungry::writer::QueuedWriter::new(w);

    let session = getrandom::u64()?.cast_signed();

    let mut sender = Sender::new(r, w, auth_key, session, server_salt);

    let (tx, mut rx) = mpsc::unbounded_channel::<Request>();

    let client = Client { tx };

    let auth_task = spawn(auth(client.clone(), api_id, api_hash, bot_auth_token));

    let sender_task = spawn(async move {
        let mut ctrl_c = pin!(tokio::signal::ctrl_c());

        let mut updates = Vec::new();

        let mut handle = Handle {};

        while let Err(err) = poll_fn::<Result<(), hungry::sender::SenderError>, _>(|cx| {
            if let Poll::Ready(ready) = ctrl_c.as_mut().poll(cx) {
                ready.unwrap();

                info!("caught `ctrl-c` notification");

                return Poll::Ready(Ok(()));
            }

            while let Poll::Ready(Some(ready)) = rx.poll_recv(cx) {
                let Request { tx, len, func } = ready;

                let _msg = sender.invoke(tx, len, |buf| buf.ser(&*func));
            }

            while let Poll::Ready(ready) = sender.poll(cx, &mut updates, &mut handle) {
                ready?;

                for update in updates.drain(..) {
                    process_updates(&client, update);
                }
            }

            Poll::Pending
        })
        .await
        {
            error!(?err);
        }

        anyhow::Ok(())
    });

    auth_task.await??;
    sender_task.await??;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stderr());

    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking));

    tracing::subscriber::set_global_default(subscriber)?;

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async_main())?;

    info!("bye");

    Ok(())
}
