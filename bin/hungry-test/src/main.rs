mod auth;
mod client;
mod handle;
mod key;
mod updates;

use std::fmt;
use std::future::poll_fn;
use std::pin::pin;
use std::task::Poll;

use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;

use hungry::unbite;

pub use auth::authorize;
pub use client::{Client, Request, RequestError};
pub use handle::Handle;
pub use key::get_auth_key;
pub use updates::process_updates;

// Test server IPv4 address.
const ADDR: &str = "149.154.167.40:443";

// The 'drivers' of the `Sender` parts.
type R = tokio::net::tcp::OwnedReadHalf;
type W = tokio::net::tcp::OwnedWriteHalf;

// See <https://core.telegram.org/mtproto/mtproto-transports> for information.
type Transport = hungry::transport::Full;

type Plain = hungry::plain::Plain<Transport, R, W>;

type Sender = hungry::sender::Sender<Transport, R, W, Handle>;

fn main() -> anyhow::Result<()> {
    let _guard = set_tracing_subscriber()?;

    tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?
        .block_on(async_main())?;

    info!("bye");

    Ok(())
}

/// Setup [`tracing`] logging with non-blocking [`std::io::stderr`] writer.
fn set_tracing_subscriber() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stderr());

    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking));

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(guard)
}

async fn async_main() -> anyhow::Result<()> {
    // Your authorization credentials for a bot on the **TEST** server.
    let api_id = std::env::var("API_ID")?.parse()?;
    let api_hash = std::env::var("API_HASH")?;
    let bot_auth_token = std::env::var("BOT_AUTH_TOKEN")?;

    let (mut plain, mut buffer) = connect().await?;

    let (auth_key, server_salt) = get_auth_key(&mut plain, &mut buffer).await?;

    let (r, w) = plain.into_inner();

    let w = hungry::writer::QueuedWriter::new(w);

    let session = getrandom::u64()?.cast_signed();

    let sender = Sender::new(r, w, auth_key, session, server_salt);

    let (tx, rx) = mpsc::unbounded_channel::<Request>();

    let client = Client { tx };

    let auth_task = spawn(authorize(client.clone(), api_id, api_hash, bot_auth_token));

    let sender_task = spawn(main_loop(sender, client, rx));

    auth_task.await??;

    sender_task.await??;

    Ok(())
}

async fn connect() -> anyhow::Result<(Plain, unbite::DynBuf)> {
    let transport = Transport::default();

    let (r, w) = tokio::net::TcpStream::connect(ADDR).await?.into_split();

    // Preallocate the buffers.
    let r_buffer = unbite::DynBuf::new(256 * 1024);
    let w_buffer = unbite::DynBuf::new(256 * 1024);

    // Initialize the `Reader` and `Writer` which are actually independent.
    let (r, w) = hungry::init(transport, r, r_buffer, w, w_buffer);

    // Initialize the transport.
    //
    // Full transport is an exception: sending a header before first
    // packet is not required, making this `.await?` absolutely free.
    let hungry::writer::OwnedWriteInner { driver: w, buffer } = w.await?;

    let plain = Plain::new(r, w);

    Ok((plain, buffer))
}

async fn main_loop(
    mut sender: Sender,
    client: Client,
    mut rx: mpsc::UnboundedReceiver<Request>,
) -> anyhow::Result<()> {
    let mut ctrl_c = pin!(tokio::signal::ctrl_c());

    let mut updates = Vec::new();

    let mut handle = Handle {};

    // This function can be thought of a loop,
    // where returning the `Pending` variant is
    // the same as using `continue` statement.
    poll_fn::<anyhow::Result<()>, _>(|cx| {
        if let Poll::Ready(ready) = ctrl_c.as_mut().poll(cx) {
            ready?;

            info!("caught `ctrl-c` notification");

            return Poll::Ready(Ok(()));
        }

        // Serialize all outgoing requests.
        while let Poll::Ready(Some(ready)) = rx.poll_recv(cx) {
            let Request { tx, len, func } = ready;

            let _msg = sender.invoke(tx, len, |buf| buf.ser(&*func));
        }

        // Poll the `Sender` and process updates.
        while let Poll::Ready(ready) = sender.poll(cx, &mut updates, &mut handle) {
            ready?;

            // Reuse `updates` vector efficiently.
            for update in updates.drain(..) {
                process_updates(&client, update);
            }
        }

        Poll::Pending
    })
    .await
}

/// Helper function to [`tokio::spawn`] with error logging.
fn spawn<E, F>(future: F) -> tokio::task::JoinHandle<Result<(), E>>
where
    E: fmt::Display + Send + 'static,
    F: Future<Output = Result<(), E>> + Send + 'static,
{
    tokio::spawn(async move { future.await.inspect_err(|err| error!(%err)) })
}
