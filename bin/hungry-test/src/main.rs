use std::future::poll_fn;
use std::task::Poll;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing_subscriber::layer::SubscriberExt;

use hungry::{crypto_bigint, tl, tracing, unbite};

use crypto_bigint::{Odd, U2048};
use tracing::info;

use tl::SerializedLen;

const ADDR: &str = "149.154.167.40:443";

const N: &str = "253428894488404155649716895907134732068988477590847790525820265945460224638539\
    4058588521595116849196570822264939918060381807420062046377613542488463216251240316379308392\
    1641631564740959529419359595852941166848940585952337613333022396096584117954892216031229237\
    3029437018775884567383353986024616752250817918203931537575049526362349513232378200365435810\
    4782690612092797248736680529211579223142368426126233039432475078545094258975175539015664775\
    1460719351439969059949569615302809050721500330239005077889855323917509948255722081644689442\
    127297605422579707142646660768825302832201908302295573257427896031830742328565032949";

type R = tokio::net::tcp::OwnedReadHalf;
type W = tokio::net::tcp::OwnedWriteHalf;

type Transport = hungry::transport::Intermediate;

type Plain = hungry::plain::Plain<Transport, R, W>;

async fn connect() -> anyhow::Result<(Plain, unbite::DynBuf)> {
    let transport = Transport::default();

    let (r, w) = tokio::net::TcpStream::connect(ADDR).await?.into_split();

    let r_buffer = unbite::DynBuf::new(16 * 1024);
    let w_buffer = unbite::DynBuf::new(16 * 1024);

    let (r, w) = hungry::init(transport, r, r_buffer, w, w_buffer);

    let hungry::writer::OwnedWriteInner { driver: w, buffer } = w.await?;

    let plain = Plain::new(r, w);

    Ok((plain, buffer))
}

async fn generate_auth_key(
    plain: &mut Plain,
    buffer: &mut unbite::DynBuf,
) -> anyhow::Result<hungry::auth::DhGenOk> {
    info!("generating new `AuthKey`");

    let n = Odd::new(U2048::from_str_radix_vartime(N, 10)?).unwrap();
    let e = Odd::new(U2048::from_word(65537)).unwrap();

    let server_public_key = hungry::crypto::RsaKey::new(n, e); // fingerprint: -5595554452916591101

    let mut nonce = tl::Int128::default();

    getrandom::fill(nonce.as_mut())?;

    let req_pq_multi = hungry::auth::start(nonce);

    let tl::mtproto::enums::ResPq::ResPq(res_pq) = plain.send(buffer, req_pq_multi.func()).await?;

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

    let tl::mtproto::enums::ServerDhParams::ServerDhParamsOk(server_dh_params) =
        plain.send(buffer, func).await?
    else {
        anyhow::bail!("not ServerDhParamsOk")
    };

    let server_dh_params = req_dh_params.server_dh_params_ok(&server_dh_params)?;

    let mut b = [0; 256];
    getrandom::fill(&mut b)?;

    let set_client_dh_params =
        server_dh_params.set_client_dh_params(U2048::from_be_slice(&b), 0)?;

    let tl::mtproto::enums::SetClientDhParamsAnswer::DhGenOk(dh_gen_ok) =
        plain.send(buffer, set_client_dh_params.func()).await?
    else {
        anyhow::bail!("not DhGenOk")
    };

    Ok(set_client_dh_params.dh_gen_ok(&dh_gen_ok)?)
}

async fn get_auth_key(
    plain: &mut Plain,
    buffer: &mut unbite::DynBuf,
) -> anyhow::Result<(hungry::mtproto::AuthKey, hungry::mtproto::Salt)> {
    let filename = "test.session";

    let path = std::path::Path::new(filename);

    let mut file = tokio::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .await?;

    let mut buf = Vec::new();

    file.read_to_end(&mut buf).await?;

    // if buf.len() != 256 {
    let hungry::auth::DhGenOk {
        auth_key,
        server_salt,
    } = generate_auth_key(plain, buffer).await?;

    info!("writing the `AuthKey` to `{filename}`");

    file.write_all(auth_key.data()).await?;

    return Ok((auth_key, server_salt));
    // }

    println!("Using the `AuthKey` from `{filename}`");

    let auth_key = hungry::mtproto::AuthKey::new(buf.try_into().unwrap()).unwrap();

    Ok((auth_key, 0))
}

async fn async_main() -> anyhow::Result<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking));

    tracing::subscriber::set_global_default(subscriber)?;

    let (mut plain, mut buffer) = connect().await?;

    let (auth_key, server_salt) = get_auth_key(&mut plain, &mut buffer).await?;

    let (r, w) = plain.into_inner();

    let w = hungry::writer::QueuedWriter::new(w);

    let session = getrandom::u64()?.cast_signed();

    let (mut sender, mut objects_rx) =
        hungry::sender::Sender::new(r, w, auth_key, session, server_salt);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(
        tokio::sync::oneshot::Sender<tokio::sync::oneshot::Receiver<tl::Object>>,
        usize,
        Box<dyn FnOnce(&mut tl::ser::Buf) + Send>,
    )>();

    let tx_clone = tx.clone();

    let task = tokio::spawn(async move {
        loop {
            poll_fn(|cx| {
                info!("polling objects");

                while let Poll::Ready(Some(ready)) = rx.poll_recv(cx) {
                    let (tx, len, f) = ready;

                    tx.send(sender.invoke(len, f)).unwrap();
                }

                while let Poll::Ready(ready) = sender.poll(cx) {
                    ready?;
                }

                while let Poll::Ready(ready) = objects_rx.poll_recv(cx) {
                    use tl::Object::*;

                    match ready.unwrap() {
                        api_Updates(tl::api::enums::Updates::Updates(updates)) => {
                            for upd in &updates.updates {
                                match upd {
                                    tl::api::enums::Update::UpdateNewMessage(msg) => match &msg
                                        .message
                                    {
                                        tl::api::enums::Message::Message(msg) => {
                                            let id = match msg.peer_id {
                                                tl::api::enums::Peer::PeerUser(ref x) => x.user_id,
                                                _ => todo!(),
                                            };

                                            let tl::api::enums::User::User(user) = updates
                                                .users
                                                .iter()
                                                .find(|x| match x {
                                                    tl::api::enums::User::UserEmpty(_) => false,
                                                    tl::api::enums::User::User(x) => x.id == id,
                                                })
                                                .unwrap()
                                            else {
                                                todo!()
                                            };

                                            let func = tl::api::funcs::messages::SendMessage {
                                                no_webpage: false,
                                                silent: false,
                                                background: false,
                                                clear_draft: false,
                                                noforwards: false,
                                                update_stickersets_order: false,
                                                invert_media: false,
                                                allow_paid_floodskip: false,
                                                peer: tl::api::types::InputPeerUser {
                                                    user_id: id,
                                                    access_hash: user.access_hash.unwrap(),
                                                }
                                                .into(),
                                                reply_to: None,
                                                message: format!("pong: {}", msg.message),
                                                random_id: getrandom::u64().unwrap().cast_signed(),
                                                reply_markup: None,
                                                entities: None,
                                                schedule_date: None,
                                                send_as: None,
                                                quick_reply_shortcut: None,
                                                effect: None,
                                                allow_paid_stars: None,
                                                suggested_post: None,
                                            };

                                            let (send_message_tx, send_message_rx) =
                                                tokio::sync::oneshot::channel();

                                            let Ok(()) = tx_clone.send((
                                                send_message_tx,
                                                func.serialized_len(),
                                                Box::new(move |buf: &mut tl::ser::Buf| {
                                                    buf.ser(&func)
                                                }),
                                            )) else {
                                                unreachable!()
                                            };

                                            tokio::spawn(async move {
                                                let _obj = dbg!(
                                                    send_message_rx.await.unwrap().await.unwrap()
                                                );
                                            });
                                        }
                                        msg => {
                                            let _ = dbg!(msg);
                                        }
                                    },
                                    upd => {
                                        let _ = dbg!(upd);
                                    }
                                }
                            }
                        }

                        object => {
                            info!(?object, "received object");
                        }
                    }
                }

                Poll::<anyhow::Result<()>>::Pending
            })
            .await?;
        }

        Ok::<(), anyhow::Error>(())
    });

    let task2 = tokio::spawn(async move {
        let func = tl::api::funcs::InvokeWithLayer {
            layer: 214,
            query: tl::api::funcs::InitConnection {
                api_id: std::env::var("API_ID")?.parse()?,
                device_model: "device_model".to_string(),
                system_version: "system_version".to_string(),
                app_version: "0.0.1".to_string(),
                system_lang_code: "en".to_string(),
                lang_pack: "".to_string(),
                lang_code: "en".to_string(),
                proxy: None,
                params: None,
                query: tl::api::funcs::auth::ImportBotAuthorization {
                    flags: 0,
                    api_id: std::env::var("API_ID")?.parse()?,
                    api_hash: std::env::var("API_HASH")?,
                    bot_auth_token: std::env::var("BOT_TOKEN")?,
                },
            },
        };

        let (import_bot_authorization_tx, import_bot_authorization_rx) =
            tokio::sync::oneshot::channel();

        let Ok(()) = tx.send((
            import_bot_authorization_tx,
            func.serialized_len(),
            Box::new(move |buf: &mut tl::ser::Buf| buf.ser(&func)),
        )) else {
            unreachable!()
        };

        let obj = import_bot_authorization_rx.await?.await?;

        let tl::Object::api_auth_Authorization(auth) = obj else {
            unimplemented!()
        };

        let tl::api::enums::auth::Authorization::Authorization(auth) = auth else {
            unimplemented!()
        };

        Ok::<(), anyhow::Error>(())
    });

    task.await??;
    task2.await??;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}
