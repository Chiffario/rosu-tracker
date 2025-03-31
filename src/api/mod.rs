use crate::websocket::structs::RespektiveUser;
use bytes::{Buf, Bytes};
use color_eyre::Result;
use http_body_util::{BodyExt, Empty};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
mod client;
pub mod respektive;

pub async fn get_respektive_user(user_id: u32) -> Result<RespektiveUser> {
    let mut url = format!("https://score.respektive.pw/u/{user_id}");
    let addr = url.clone();
    let req = hyper::Request::builder()
        .uri(url)
        .body(Empty::<Bytes>::new())?;
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    let mut res = sender.send_request(req).await?;

    let body = res.collect().await?.aggregate();

    let user = serde_json::from_reader(body.reader())?;

    Ok(user)
}
