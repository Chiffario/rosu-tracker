use std::io::Cursor;

use color_eyre::Result;
use cosmic::iced_futures;
use cosmic::iced_futures::subscription::Event;
use cosmic::iced_wgpu::graphics::image::image_rs::{DynamicImage, ImageReader};
use futures_util::{SinkExt, Stream};
use http_body_util::{BodyExt, Collected, Empty};
use hyper::body::Bytes;
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use image;
use tokio::net::TcpStream;
use tokio::stream;

use crate::gui::app::AppMessage;

const BASE_COVER_URL: &str = "http://assets.ppy.sh/beatmaps/";
const END_COVER_URL: &str = "/covers/cover.jpg";

async fn fetch_url(url: hyper::Uri) -> Result<Collected<Bytes>> {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let authority = url.authority().unwrap().clone();

    let path = url.path();
    let req = Request::builder()
        .uri(path)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    let mut res = sender.send_request(req).await?;
    if res.status() == StatusCode::from_u16(404).unwrap() {
        return Err(color_eyre::Report::msg("Image not found"));
    }

    // println!("Response: {}", res.status());
    tracing::debug!("Headers: {:#?}\n", res.headers());

    let output = res.collect().await;

    Ok(output?)
}

fn form_url(beatmapset_id: u32) -> hyper::Uri {
    let uri_string = format!("{}{}{}", BASE_COVER_URL, beatmapset_id, END_COVER_URL);
    uri_string.parse::<hyper::Uri>().unwrap()
}

fn parse_image(bytes: Collected<Bytes>) -> Option<DynamicImage> {
    let res = bytes.to_bytes();

    let mut image = ImageReader::new(Cursor::new(res));
    image.set_format(image::ImageFormat::Jpeg);
    let image = image.decode().ok();
    image
}

pub fn fetch_multiple(id_list: Box<[u32]>) -> impl Stream<Item = cosmic::app::Message<AppMessage>> {
    iced_futures::stream::channel(100, move |mut output| async move {
        for id in id_list {
            let url = form_url(id);
            let image = fetch_url(url).await.ok();
            let i = image.and_then(|i| parse_image(i));
            let _ = output
                .send(cosmic::app::Message::App(AppMessage::ReceiveBackground(
                    id, i,
                )))
                .await;
        }
    })
}
