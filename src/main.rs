use async_io::block_on;
use futures_util::{SinkExt, StreamExt};
use rosu_v2::prelude::*;
use serde::Deserialize;
use std::arch::x86_64::_mm_cmplt_epi16;
use std::sync::Arc;
use std::{fs::File, io::read_to_string, thread, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
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
    let initial_data = Arc::new(TrackedData::new_with(
        osu.user(&api_conf.username).await.ok(),
        osu.user_scores(&api_conf.username).await.ok(),
        osu.user_scores(&api_conf.username).firsts().await.ok(),
    ));
    tracing::debug!(
        "Fetched initial data: {:?}",
        initial_data.user_extended.as_ref().unwrap().username
    );
    let tracked_data: Arm<TrackedData> = Arc::new(Mutex::new(TrackedData::new_with(
        initial_data.user_extended.clone(),
        initial_data.user_scores.clone(),
        initial_data.user_firsts.clone(),
    )));
    let alt_clients = clients.clone();
    // Setup a thread to actually serve the data
    let tracker = tracked_data.clone();
    let client_thread = tokio::spawn(async move {
        loop {
            handle_clients(alt_clients.clone(), tracker.clone()).await;
            time::sleep(Duration::from_secs(3)).await;
        }
    });
    println!("Spawned client thread");
    // Setup thread to fetch data from osu api
    let tracker = tracked_data.clone();
    let fetch_thread = tokio::spawn(async move {
        let osu_ref = osu.clone();
        let init_data = initial_data.clone();
        let tracker = tracker.clone();
        let config = api_conf.clone();
        fetch_thread(osu_ref, init_data, tracker, config).await;
    });
    println!("Spawned fetch thread");
    // Setup a thread to run the server
    let tracker = tracked_data.clone();
    let server_thread = tokio::spawn(async { server_thread(clients, tracker).await });
    println!("Spawned server thread");
    let _ = fetch_thread.await;
    let _ = server_thread.await;
    let _ = client_thread.await;
    Ok(())
}
