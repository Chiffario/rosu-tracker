use color_eyre::eyre::{Report, eyre};
use color_eyre::{Result, eyre};
// use cosmic::cosmic_config::{self, ConfigGet};
use std::sync::Arc;
use tokio::sync::Mutex;

use rosu_v2::Osu;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use crate::structs::{Arm, Clients, TrackedData};
use crate::{fetch_thread, handle_clients, server_thread};
use types::Api;
use types::Either;

pub async fn thread_init(config: Either<Api, fn() -> Result<Api>>) -> Result<()> {
    // Setup tracing
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let clients = Clients::default();
    let api_conf = match config {
        Either::Left(api) => api,
        Either::Right(fun) => fun()?,
    };
    // Prep empty websocket clients
    debug!("Constructed clients");
    let osu = Arc::new(
        Osu::new(api_conf.id.parse().unwrap(), &api_conf.secret)
            .await
            .map_err(|e| eyre!("Failed to initialise osu client: {e}"))?,
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
    let user = osu_user
        .await
        .inspect_err(|e| error!("failed to fetch user: {}", e))
        .ok();
    let scores = osu_user_scores
        .await
        .inspect_err(|e| error!("failed to fetch top scores: {}", e))
        .ok();
    let firsts = osu_user_firsts
        .await
        .inspect_err(|e| error!("failed to fetch first place scores: {}", e))
        .ok();
    let recent = osu_user_recent
        .await
        .inspect_err(|e| error!("failed to fetch recent scores: {}", e))
        .ok();
    tracked_data
        .lock()
        .await
        .insert(user, scores, firsts, recent);
    debug!("Spawned server thread");
    let _ = tokio::join!(fetch_thread, server_thread, client_thread);
    Ok(())
}
