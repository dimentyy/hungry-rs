use std::sync::Arc;

use tracing::info;

use hungry::tl;

use tl::api::{enums, funcs, types};

use crate::{Client, spawn};

fn send_message(peer: enums::InputPeer, message: String) -> funcs::messages::SendMessage {
    let random_id = getrandom::u64().unwrap().cast_signed();

    funcs::messages::SendMessage {
        no_webpage: false,
        silent: false,
        background: false,
        clear_draft: false,
        noforwards: false,
        update_stickersets_order: false,
        invert_media: false,
        allow_paid_floodskip: false,
        peer,
        reply_to: None,
        message,
        random_id,
        reply_markup: None,
        entities: None,
        schedule_date: None,
        send_as: None,
        quick_reply_shortcut: None,
        effect: None,
        allow_paid_stars: None,
        suggested_post: None,
    }
}

pub fn process_updates(client: &Client, updates: enums::Updates) {
    match updates {
        enums::Updates::Updates(updates) => {
            for update in &updates.updates {
                match update {
                    enums::Update::UpdateNewMessage(update) => match &update.message {
                        enums::Message::Message(message) => {
                            let id = match message.peer_id {
                                enums::Peer::PeerUser(ref x) => x.user_id,
                                _ => todo!(),
                            };

                            let enums::User::User(user) = updates
                                .users
                                .iter()
                                .find(|x| match x {
                                    enums::User::UserEmpty(_) => false,
                                    enums::User::User(x) => x.id == id,
                                })
                                .unwrap()
                            else {
                                todo!()
                            };

                            let input_peer = types::InputPeerUser {
                                user_id: id,
                                access_hash: user.access_hash.unwrap_or(0),
                            };

                            let message = if message.message == "/stats" {
                                let mut sys = sysinfo::System::new();
                                let pid = sysinfo::Pid::from(std::process::id() as usize);
                                let pids = sysinfo::ProcessesToUpdate::Some(&[pid]);

                                sys.refresh_processes_specifics(
                                    pids,
                                    false,
                                    sysinfo::ProcessRefreshKind::nothing()
                                        .with_cpu()
                                        .with_memory(),
                                );

                                let proc = sys.process(pid).unwrap();

                                let run_time = proc.run_time();
                                let cpu_time = proc.accumulated_cpu_time() as f64 / 1000.;
                                let cpu_usage = cpu_time / run_time as f64 * 100.;
                                let mem = proc.memory() as f64 / (1024. * 1024.);

                                format!(
                                    "# STATS:\n\n * Run time: {run_time}s\n * CPU: {cpu_time:.2}s ({cpu_usage:.3}%)\n * Memory: {mem:.2}MiB"
                                )
                            } else {
                                format!("echo: {}", message.message)
                            };

                            let func = Arc::new(send_message(input_peer.into(), message));

                            let client = client.clone();

                            spawn(async move {
                                let _obj = client.invoke(func).await?;

                                anyhow::Ok(())
                            });
                        }
                        message => {
                            info!(?message);
                        }
                    },
                    update => {
                        info!(?update);
                    }
                }
            }
        }
        updates => {
            info!(?updates);
        }
    }
}
