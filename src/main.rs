use futures_util::{future, SinkExt, StreamExt, TryStreamExt};
use rosu_v2::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use std::{borrow::Borrow, fs::File, io::Read, thread, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Deserialize)]
struct Api {
    id: String,
    secret: String,
}
#[tokio::main]
async fn main() {
    let mut buf = String::new();
    let config = File::open("config.toml").unwrap().read_to_string(&mut buf);
    let api_conf: Api = toml::from_str(&buf).unwrap();

    let osu = Osu::new(api_conf.id.parse().unwrap(), api_conf.secret)
        .await
        .unwrap();

    let osu_ptr = Arc::new(osu);

    let addr = "127.0.0.1:7272".to_string();
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    while let Ok((stream, _)) = listener.accept().await {
        let ptr = osu_ptr.clone();
        tokio::spawn(async {
            accept_connection(stream, ptr).await;
        });
    }
}

async fn accept_connection(stream: TcpStream, osu: Arc<Osu>) {
    let addr = stream
        .peer_addr()
        .expect("Connected streams should have a peer address");
    let ws_stream = accept_async(stream).await.expect("Error during handshake");

    let (mut write, read) = ws_stream.split();
    let scores: Vec<Score> = osu
        .user_scores("Chiffa")
        .mode(GameMode::Osu)
        .best()
        .await
        .unwrap();
    loop {
        let w = write
            .send(Message::Text(serde_json::to_string(&scores).unwrap()))
            .await;
        match w {
            Ok(wr) => thread::sleep(Duration::from_secs(1)),
            Err(e) => {
                eprintln!("{e}");
                break;
            }
        }
    }
}
