use std::borrow::Borrow;
use std::{sync::Arc, thread, time::Duration};

use futures_util::{SinkExt, StreamExt};
use hyper::upgrade::Upgraded;
use hyper::Request;
use hyper::{service::service_fn, Response};
use hyper_util::rt::tokio::TokioIo;
use rosu_v2::{model::GameMode, prelude::UserExtended, Osu};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{ClientHandshake, Message},
    WebSocketStream,
};

use crate::{Api, TrackedData};

pub async fn serve_profile(
    client: WebSocketStream<TokioIo<Upgraded>>,
    request: Request<hyper::body::Incoming>,
) -> Result<Response<String>, hyper::Error> {
    Ok(Response::new("".to_string()))
}
pub async fn accept_connection(
    stream: TcpStream,
    osu: Arc<Osu>,
    initial_data: Arc<TrackedData>,
    tracked_data: Arc<Mutex<TrackedData>>,
    api_conf: Api,
) {
    let addr = stream
        .peer_addr()
        .expect("Connected streams should have a peer address");
    let ws_stream = accept_async(stream).await.expect("Error during handshake");
    let (mut write, read) = ws_stream.split();
    loop {
        let user = osu.user(&api_conf.username).await.unwrap();
        let mut tracked = tracked_data.lock().await;
        if user != tracked.user_extended {
            if user.statistics.as_ref().unwrap().pp
                != tracked.user_extended.statistics.as_ref().unwrap().pp
            {
                let new_tops = osu.user_scores(&api_conf.username).await.unwrap();
                tracked.user_scores = new_tops;
            }
            if user.scores_first_count != tracked.user_extended.scores_first_count {
                let new_firsts = osu.user_scores(&api_conf.username).firsts().await.unwrap();
                tracked.user_firsts = new_firsts;
            }
            tracked.user_extended = user;
            // match w {
            //     Ok(wr) => thread::sleep(Duration::from_secs(15)),
            //     Err(e) => {
            //         eprintln!("{e}");
            //         break;
            //     }
            // }
        }
    }
}
