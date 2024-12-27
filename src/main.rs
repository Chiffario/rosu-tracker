use rosu_v2::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use std::{fs::File, io::read_to_string};
use tokio::sync::Mutex;
mod websocket;
use websocket::{fetch_thread, server_thread};
use websocket::{handle_clients, structs::*};
#[derive(Deserialize, Clone)]
struct Api {
    id: String,
    secret: String,
    port: String,
    username: String,
}
#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Setup tracing
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;
    // Parse user's configuration
    let config = read_to_string(File::open("config.toml").unwrap()).unwrap();
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
    let _ = fetch_thread.await;
    let _ = server_thread.await;
    let _ = client_thread.await;
    Ok(())
}
