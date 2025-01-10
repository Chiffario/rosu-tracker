use cosmic::cosmic_config;
use directories::BaseDirs;
use eyre::eyre;
use std::{
    fs::File,
    io::{self, read_to_string, Write},
    path::{Path, PathBuf},
    sync::{self, Arc, LazyLock},
};
use tokio::sync::Mutex;

use rosu_v2::Osu;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::websocket::{
    fetch_thread, handle_clients, server_thread,
    structs::{Arm, Clients, TrackedData},
};

static CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    BaseDirs::new()
        .expect("Please create a configuration directory")
        .config_local_dir()
        .to_path_buf()
        .join("rosu-tracker")
        .join("config.toml")
});

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Api {
    pub id: String,
    pub secret: String,
    pub username: String,
}

impl Api {
    pub(crate) fn write(&self) -> eyre::Result<()> {
        let config = toml::to_string(self).map_err(eyre::Error::new)?;
        let mut f = File::create(CONFIG_DIR.as_path()).map_err(eyre::Error::new)?;
        f.write(config.as_bytes()).map_err(eyre::Error::new)?;
        Ok(())
    }
}

pub async fn thread_init() -> eyre::Result<()> {
    // Setup tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    // Parse user's configuration
    let config = read_to_string(
        File::open(CONFIG_DIR.as_path()).map_err(|e| eyre!("Couldn't open a file: {e}"))?,
    )
    .map_err(|e| eyre!("Couldn't read a file: {e}"))?;
    let api_conf: Api = toml::from_str(&config).map_err(|e| eyre!("Malformed config file: {e}"))?;
    println!("Configuration constructed");

    let clients = Clients::default();
    // Prep empty websocket clients
    tracing::debug!("Constructed clients");
    println!("{CONFIG_DIR:?}");
    let osu = Arc::new(
        Osu::new(api_conf.id.parse().unwrap(), &api_conf.secret)
            .await
            .unwrap(),
    );

    let osu_user = osu.user(&api_conf.username);
    let osu_user_scores = osu.user_scores(&api_conf.username).limit(100);
    let osu_user_firsts = osu.user_scores(&api_conf.username).firsts();
    let osu_user_recent = osu.user_scores(&api_conf.username).recent().limit(20);
    let tracked_data: Arm<TrackedData> = Arc::new(Mutex::new(TrackedData::new()));
    let alt_clients = clients.clone();
    // Setup a thread to actually serve the data
    let tracker = tracked_data.clone();
    let client_thread = tokio::spawn(async move {
        loop {
            handle_clients(alt_clients.clone(), tracker.clone()).await;
        }
    });
    debug!("Spawned client thread");
    // Setup thread to fetch data from osu api
    let tracker = tracked_data.clone();
    let osu_ref = osu.clone();
    let fetch_thread = tokio::spawn(async move {
        let tracker = tracker.clone();
        let config = api_conf.clone();
        fetch_thread(osu_ref, tracker, config).await;
    });
    debug!("Spawned fetch thread");
    // Setup a thread to run the server
    let tracker = tracked_data.clone();
    let server_thread = tokio::spawn(async { server_thread(clients, tracker).await });
    let user = osu_user.await.ok();
    let scores = osu_user_scores.await.ok();
    let firsts = osu_user_firsts.await.ok();
    let recent = osu_user_recent.await.ok();
    tracked_data
        .lock()
        .await
        .insert(user, scores, firsts, recent);
    debug!("Spawned server thread");
    let _ = fetch_thread.await;
    let _ = server_thread.await;
    let _ = client_thread.await;
    // Ok(handles)
    Ok(())
}
