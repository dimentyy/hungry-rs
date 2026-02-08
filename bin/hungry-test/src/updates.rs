use std::slice;
use std::sync::Arc;

use tracing::{info, warn};

use hungry::tl;

use tl::api::{enums, funcs, types};

use crate::{Client, spawn};

pub fn process_updates(client: &Client, updates: enums::Updates) {
    use enums::Updates::*;

    // Flatten `Updates` into a slice.
    let updates = match updates {
        UpdateShort(x) => vec![x.update],
        UpdatesCombined(x) => x.updates,
        Updates(x) => x.updates,
        updates => {
            warn!(?updates, "unsupported updates");
            return;
        }
    };

    for update in updates {
        process_update(client, update);
    }
}

fn process_update(client: &Client, update: enums::Update) {
    use enums::Update::*;

    info!(?update);

    match update {
        UpdateNewMessage(update) => match update.message {
            enums::Message::Message(message) => {
                spawn(process_message(client.clone(), *message));
            }
            _ => {}
        },
        _ => {}
    }
}

async fn process_message(client: Client, message: types::Message) -> anyhow::Result<()> {
    let id = match message.peer_id {
        enums::Peer::PeerUser(ref x) => x.user_id,
        _ => anyhow::bail!("unsupported peer"),
    };

    let input_peer = types::InputPeerUser {
        user_id: id,

        // Server seems to accept the empty `access_hash` field,
        // but for real projects you should cache it from updates.
        access_hash: 0,
    };

    let message = if message.message == "/stats" {
        stats_message()
    } else {
        format!("echo: {}", message.message)
    };

    let func = Arc::new(send_message(input_peer.into(), message));

    let client = client.clone();

    let _obj = client.invoke(func).await?;

    Ok(())
}

fn stats_message() -> String {
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
}

fn send_message(peer: enums::InputPeer, message: String) -> funcs::messages::SendMessage {
    let random_id = getrandom::u64().unwrap().cast_signed();

    // TODO: macros for providing default values.
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
