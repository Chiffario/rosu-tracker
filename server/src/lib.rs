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
use tokio::time::sleep;
use tokio_tungstenite::{
    tungstenite::{handshake::derive_accept_key, protocol::Role, Message},
    WebSocketStream,
};
use tracing::{debug, error};
pub mod structs;
pub mod setup;

use constants::{BASE_IP, FIRSTS_ENDPOINT, RECENT_ENDPOINT, TOPS_ENDPOINT, USER_ENDPOINT};
use types::Api;
use color_eyre::{eyre::Error, Result};
use tokio_tungstenite::tungstenite::Utf8Bytes;
use structs::*;

#[tracing::instrument(name = "handle_clients", skip_all)]
pub async fn handle_clients(clients: Clients, values: Arm<TrackedData>) {
    let user_lock = values.lock().await;
    if user_lock.user_extended.is_none() {
        return;
    }
    let (ser_profile, ser_tops, ser_firsts, ser_recent) = {
        let data = &*user_lock;
        (
            serde_json::to_string(&data.user_extended).unwrap(),
            serde_json::to_string(&data.user_scores).unwrap(),
            serde_json::to_string(&data.user_firsts).unwrap(),
            serde_json::to_string(&data.user_recent).unwrap(),
        )
    };
    drop(user_lock);
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
                WsKind::User => socket.client.send(Message::Text(Utf8Bytes::from(ser_profile.clone()))).await,
                WsKind::Tops => socket.client.send(Message::Text(Utf8Bytes::from(ser_tops.clone()))).await,
                WsKind::Firsts => socket.client.send(Message::Text(Utf8Bytes::from(ser_firsts.clone()))).await,
                WsKind::Recent => socket.client.send(Message::Text(Utf8Bytes::from(ser_recent.clone()))).await,
            };
            debug!("Sent data to {:?}", socket);
            // Close the connection on error
            // Panics are bad, leaving it as it is even worse
            if let Err(e) = res {
                println!("{:?}", e);

                let _ = socket.client.send(Message::Close(None)).await;
                return false;
            };

            true
        })
    });
    drop(clients);
    tokio::time::sleep(Duration::from_secs(1)).await;
}
#[tracing::instrument(name = "server_thread")]
pub async fn server_thread(ctx_clients: Clients, values: Arm<TrackedData>) {
    let tcp = tokio::net::TcpListener::bind(BASE_IP).await.unwrap();
    loop {
        let (stream, _) = tcp.accept().await.unwrap();

        let io = TokioIo::new(stream);

        let ctx_clients = ctx_clients.clone();
        let _ctx_values = values.clone();
        let service = service_fn(move |req| {
            let ctx_clients = ctx_clients.clone();
            // let ctx_values = ctx_values.clone();
            // serve(ctx_clients, ctx_values, req)
            serve(ctx_clients, req)
        });
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
#[tracing::instrument(name = "fetch_thread", skip_all)]
pub async fn fetch_thread(osu: Arc<Osu>, tracked_data: Arm<TrackedData>, api_conf: Api) {
    loop {
        let fetched_recent = osu.user_scores(&api_conf.username).recent().limit(20);
        let fetched_user = osu.user(&api_conf.username).await;
        match &fetched_user {
            Ok(u) => debug!("Fetched: {}", u.username),
            Err(e) => error!("Error: {e}"),
        };
        let fetched_user = fetched_user.unwrap();

        let mut tracked_data = tracked_data.lock().await;
        if let Some(ref tracked_data_user) = tracked_data.user_extended {
            if tracked_data_user.statistics != fetched_user.statistics {
                debug!("User data changed, fetching new data");
                let fetched_tops = osu.user_scores(&api_conf.username).limit(100);
                let fetched_firsts = osu.user_scores(&api_conf.username).firsts();
                tracked_data.user_scores = fetched_tops
                    .await
                    .inspect_err(|e| tracing::error!("{e}"))
                    .ok();
                tracked_data.user_firsts = fetched_firsts
                    .await
                    .inspect_err(|e| tracing::error!("{e}"))
                    .ok();
            }
            tracked_data.user_extended = Some(fetched_user);
            tracked_data.user_recent = Some(fetched_recent.await.unwrap_or_default());
            drop(tracked_data);
            let _ = sleep(Duration::from_secs(5)).await;
        } else {
            tracing::debug!("Tracked user has no data");
            let _ = sleep(Duration::from_secs(1)).await;
        }
    }
}
async fn serve(
    clients: Clients,
    // values: Arm<TrackedData>,
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>> {
    debug!("Called with uri {}", req.uri());
    match req.uri().path() {
        FIRSTS_ENDPOINT => serve_ws(clients, req, WsKind::Firsts).await,
        TOPS_ENDPOINT => serve_ws(clients, req, WsKind::Tops).await,
        USER_ENDPOINT => serve_ws(clients, req, WsKind::User).await,
        RECENT_ENDPOINT => serve_ws(clients, req, WsKind::Recent).await,
        _ => {
            println!("This URI doesn't exist");
            Err(Error::msg("This URI doesn't exist"))
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
