use futures_util::{SinkExt, StreamExt};
use rosu_v2::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use std::{fs::File, io::read_to_string, thread, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

mod websocket;
use websocket::accept_connection;
#[derive(Deserialize, Clone)]
struct Api {
    id: String,
    secret: String,
    port: String,
    username: String,
}
#[derive(Clone)]
struct TrackedData {
    user_extended: UserExtended,
    user_scores: Vec<Score>,
    user_firsts: Vec<Score>,
    // TODO: Add scorefarm api
}
#[tokio::main]
async fn main() {
    let config = read_to_string(File::open("config.toml").unwrap()).unwrap();
    let api_conf: Api = toml::from_str(&config).unwrap();

    let osu = Arc::new(
        Osu::new(api_conf.id.parse().unwrap(), &api_conf.secret)
            .await
            .unwrap(),
    );

    let initial_data = Arc::new(TrackedData {
        user_extended: osu.user(&api_conf.username).await.unwrap(),
        user_scores: osu.user_scores(&api_conf.username).await.unwrap(),
        user_firsts: osu.user_scores(&api_conf.username).firsts().await.unwrap(),
    });

    let tracked_data: Arc<Mutex<TrackedData>> = Arc::new(Mutex::new(TrackedData {
        user_extended: initial_data.user_extended.clone(),
        user_scores: initial_data.user_scores.clone(),
        user_firsts: initial_data.user_firsts.clone(),
    }));

    let addr = format!("127.0.0.1:{}", api_conf.port).to_string();
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    while let Ok((stream, _)) = listener.accept().await {
        let ptr = osu.clone();
        let initial_data_ref = initial_data.clone();
        let tracked_data_ref = tracked_data.clone();
        let config = api_conf.clone();
        tokio::spawn(async {
            accept_connection(stream, ptr, initial_data_ref, tracked_data_ref, config).await;
        });
    }
}
