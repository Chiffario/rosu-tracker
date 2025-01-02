use std::{fs::File, io::read_to_string, sync::Arc};
use tokio::{sync::Mutex, task::AbortHandle};

use rosu_v2::Osu;
use serde::Deserialize;
use tracing::{level_filters::LevelFilter, Level};
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, util::SubscriberInitExt};

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

pub async fn thread_init() -> eyre::Result<Vec<AbortHandle>> {
    // Setup tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    // Parse user's configuration
    let config =
        read_to_string(File::open("/home/chiffa/Dev/Projects/rosu-tracker/config.toml").unwrap())
            .unwrap();
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
    let osu_user_scores = osu.user_scores(&api_conf.username);
    let osu_user_firsts = osu.user_scores(&api_conf.username);
    let tracked_data: Arm<TrackedData> = Arc::new(Mutex::new(TrackedData::new()));
    let alt_clients = clients.clone();
    // Setup a thread to actually serve the data
    let tracker = tracked_data.clone();
    let client_thread = tokio::spawn(async move {
        loop {
            handle_clients(alt_clients.clone(), tracker.clone()).await;
        }
    });
    println!("Spawned client thread");
    // Setup thread to fetch data from osu api
    let tracker = tracked_data.clone();
    let osu_ref = osu.clone();
    let fetch_thread = tokio::spawn(async move {
        let tracker = tracker.clone();
        let config = api_conf.clone();
        fetch_thread(osu_ref, tracker, config).await;
    });
    println!("Spawned fetch thread");
    // Setup a thread to run the server
    let tracker = tracked_data.clone();
    let server_thread = tokio::spawn(async { server_thread(clients, tracker).await });
    let user = osu_user.await.ok();
    let scores = osu_user_scores.await.ok();
    let firsts = osu_user_firsts.await.ok();
    tracked_data.lock().await.insert(user, scores, firsts);
    println!("Spawned server thread");
    let handles = vec![
        fetch_thread.abort_handle(),
        server_thread.abort_handle(),
        client_thread.abort_handle(),
    ];
    Ok(handles)
}
