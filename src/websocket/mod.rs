use futures_lite::future::*;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{
    body::Bytes,
    header::{HeaderValue, CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, UPGRADE},
    server::conn::http1,
    service::service_fn,
    Request, Response, StatusCode,
};
use hyper_util::rt::tokio::TokioIo;
use rosu_v2::Osu;
use std::{sync::Arc, time::Duration};
use tokio::{net::TcpListener, spawn, task::block_in_place, time::sleep};
use tokio_tungstenite::{
    tungstenite::{handshake::derive_accept_key, protocol::Role, Message},
    WebSocketStream,
};
use tracing::{debug, info};
pub mod structs;
use crate::Api;
use eyre::Result;
use structs::*;

#[tracing::instrument(name = "handle_clients", skip_all)]
pub async fn handle_clients(clients: Clients, values: Arm<TrackedData>) {
    let (ser_profile, ser_tops, ser_firsts) = {
        let lock = values.lock().await;
        let data = &*lock;
        (
            serde_json::to_string(&data.user_extended).unwrap(),
            serde_json::to_string(&data.user_scores).unwrap(),
            serde_json::to_string(&data.user_firsts).unwrap(),
        )
    };
    debug!("Constructed serialized data");
    let mut clients = clients.lock().await;
    clients.retain_mut(|socket| {
        block_on(async {
            let next = socket.client.next();

            let message = futures_lite::future::poll_once(next);

            let msg = match message.await {
                Some(Some(Ok(v))) => Some(v),
                Some(Some(Err(_))) => return false,
                Some(None) | None => None,
            };

            if let Some(Message::Close(_)) = msg {
                return false;
            }

            let res = match socket.kind {
                WsKind::User => socket.client.send(Message::Text(ser_profile.clone())).await,
                WsKind::Tops => socket.client.send(Message::Text(ser_tops.clone())).await,
                WsKind::Firsts => socket.client.send(Message::Text(ser_firsts.clone())).await,
            };
            debug!("Sent data to {:?}", socket);
            // Close the connection on error
            // Panics are bad, leaving it as is is even worse
            if let Err(e) = res {
                println!("{:?}", e);

                let _ = socket.client.send(Message::Close(None)).await;
                return false;
            };

            true
        })
    });
}
#[tracing::instrument(name = "server_thread")]
pub async fn server_thread(ctx_clients: Clients, values: Arm<TrackedData>) {
    println!("websockets::server_thread()");
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:7272")
        .await
        .unwrap();
    println!("server_thread: listener constructed");
    loop {
        let (stream, _) = tcp.accept().await.unwrap();

        let io = TokioIo::new(stream);

        let ctx_clients = ctx_clients.clone();
        let ctx_values = values.clone();
        println!("server_thread: service constructed");
        let service = service_fn(move |req| {
            let ctx_clients = ctx_clients.clone();
            let ctx_values = ctx_values.clone();
            serve(ctx_clients, ctx_values, req)
        });
        println!("start building http clients");
        tokio::spawn(async {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
#[tracing::instrument(
    name = "fetch_thread",
    skip(osu, api_conf, _initial_data, tracked_data)
)]
pub async fn fetch_thread(
    osu: Arc<Osu>,
    _initial_data: Arc<TrackedData>,
    tracked_data: Arm<TrackedData>,
    api_conf: Api,
) {
    println!("websockets::fetch_thread()");
    loop {
        let user = osu.user(&api_conf.username).await;
        match &user {
            Ok(u) => println!("Fetched user: {}", u.username),
            Err(e) => eprintln!("Error: {e}"),
        };
        let user = user.unwrap();

        let mut tracked = tracked_data.lock().await;
        // TODO: User comparisons are apparently unreliable!
        if &user != tracked.user_extended.as_ref().unwrap() {
            println!("Different data!");
            if user.statistics.as_ref().unwrap().pp
                != tracked
                    .user_extended
                    .as_ref()
                    .unwrap()
                    .statistics
                    .as_ref()
                    .unwrap()
                    .pp
            {
                let new_tops = osu.user_scores(&api_conf.username).await.unwrap();
                tracked.user_scores.replace(new_tops);
            }
            if user.scores_first_count != tracked.user_extended.as_ref().unwrap().scores_first_count
            {
                let new_firsts = osu.user_scores(&api_conf.username).firsts().await.unwrap();
                tracked.user_firsts.replace(new_firsts);
            }
            tracked.user_extended = Some(user);
        } else {
            println!("Same data!");
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}
async fn serve(
    clients: Clients,
    values: Arm<TrackedData>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>> {
    debug!("Called with uri {}", req.uri());
    match req.uri().path() {
        "/firsts" => serve_ws(clients, req, WsKind::Firsts).await,
        "/tops" => serve_ws(clients, req, WsKind::Tops).await,
        "/" => serve_ws(clients, req, WsKind::User).await,
        _ => {
            println!("This URI doesn't exist");
            Err(eyre::Error::msg("This URI doesn't exist"))
        }
    }
}
#[tracing::instrument(name = "serve_ws")]
async fn serve_ws(
    clients: Clients,
    mut req: Request<hyper::body::Incoming>,
    kind: WsKind,
) -> Result<Response<Full<Bytes>>> {
    println!("websockets::serve_ws()");
    let headers = req.headers();
    let key = headers.get(SEC_WEBSOCKET_KEY);
    let derived = key.map(|k| derive_accept_key(k.as_bytes()));
    let ver = req.version();

    tokio::spawn(async move {
        let upgraded = hyper::upgrade::on(&mut req).await.expect("Upgrade failed!");

        let upgraded = TokioIo::new(upgraded);

        let client = WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;

        let ws_client = WsClient { client, kind };

        let mut clients = clients.lock().await;

        clients.push(ws_client);
    });

    let mut res = Response::new(Full::new(Bytes::default()));

    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *res.version_mut() = ver;

    res.headers_mut()
        .append(CONNECTION, HeaderValue::from_static("Upgrade"));

    res.headers_mut()
        .append(UPGRADE, HeaderValue::from_static("websocket"));

    res.headers_mut().append(
        SEC_WEBSOCKET_ACCEPT,
        derived.unwrap().parse().unwrap(), //TODO remove unwraps
    );

    Ok(res)
}
