use directories::BaseDirs;
use eyre::eyre;
use std::{fs::File, io::read_to_string, sync::Arc};
use tokio::sync::Mutex;

use rosu_v2::Osu;
use serde::Deserialize;
use tracing::debug;

use crate::websocket::{
    fetch_thread, handle_clients, server_thread,
    structs::{Arm, Clients, TrackedData},
};

#[derive(Deserialize, Clone)]
pub struct Api {
    pub id: String,
    pub secret: String,
    pub username: String,
}

pub async fn thread_init() -> eyre::Result<()> {
    // pub async fn thread_init() -> eyre::Result<Vec<AbortHandle>> {
    // Setup tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    // Parse user's configuration
    let mut path = match BaseDirs::new() {
        Some(dir) => dir.config_local_dir().to_owned(),
        None => {
            // TODO: Give an example path
            return Err(eyre!("Please create a configuration directory"));
        }
    };
    path.push("rosu-tracker/");
    path.push("config.toml");

    let config = read_to_string(File::open(path).unwrap()).unwrap();
    let api_conf: Api = toml::from_str(&config).unwrap();
    println!("Configuration constructed");

    let clients = Clients::default();
    // Prep empty websocket clients
    tracing::debug!("Constructed clients");

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
