use cosmic::iced::{
    futures::{
        channel::mpsc::{self, Receiver},
        SinkExt,
    },
    stream,
};
use futures_util::{Stream, StreamExt, TryStreamExt};
use hyper::Response;
use rosu_v2::prelude::{Score, UserExtended};
use tokio::{net::TcpStream, select};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub fn connect_tops() -> impl Stream<Item = Event> {
    let uri = "ws://127.0.0.1:7272/tops";
    println!("connect_tops called");
    stream::channel(100, move |mut output| async move {
        let mut state = State::Disconnected;

        loop {
            match &mut state {
                State::Disconnected => match connect_async(uri).await {
                    Ok((websocket, _)) => {
                        let (sender, receiver) = mpsc::channel(100);

                        let _ = output.send(Event::Connected(Connection(sender))).await;

                        state = State::Connected(websocket, receiver);
                    }
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        let _ = output.send(Event::Disconnected).await;
                    }
                },
                State::Connected(websocket, input) => {
                    let (write, read) = futures_util::StreamExt::split(websocket);
                    let fe = read.for_each(|message| {
                        let mut value = output.clone();
                        async move {
                            if let Ok(data) = message {
                                let data = data.to_text();
                                if let Ok(data) = data {
                                    let user: Result<Vec<Score>, serde_json::Error> =
                                        serde_json::from_str(data);
                                    if let Ok(user) = user {
                                        let _ = value
                                            .send(Event::MessageReceived(Message::Firsts(user)))
                                            .await;
                                    }
                                }
                            }
                        }
                    });
                    fe.await;
                }
            }
        }
    })
}

pub fn connect_firsts() -> impl Stream<Item = Event> {
    let uri = "ws://127.0.0.1:7272/firsts";
    println!("connect_firsts called");
    stream::channel(100, move |mut output| async move {
        let mut state = State::Disconnected;

        loop {
            match &mut state {
                State::Disconnected => match connect_async(uri).await {
                    Ok((websocket, _)) => {
                        let (sender, receiver) = mpsc::channel(100);

                        let _ = output.send(Event::Connected(Connection(sender))).await;

                        state = State::Connected(websocket, receiver);
                    }
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        let _ = output.send(Event::Disconnected).await;
                    }
                },
                State::Connected(websocket, input) => {
                    let (write, read) = futures_util::StreamExt::split(websocket);
                    let fe = read.for_each(|message| {
                        let mut value = output.clone();
                        async move {
                            if let Ok(data) = message {
                                let data = data.to_text();
                                if let Ok(data) = data {
                                    let user: Result<Vec<Score>, serde_json::Error> =
                                        serde_json::from_str(data);
                                    if let Ok(user) = user {
                                        let _ = value
                                            .send(Event::MessageReceived(Message::Firsts(user)))
                                            .await;
                                    }
                                }
                            }
                        }
                    });
                    fe.await;
                }
            }
        }
    })
}
pub fn connect_user() -> impl Stream<Item = Event> {
    let uri = "ws://127.0.0.1:7272/";
    println!("connect_user called");
    // FIXME: Websocket disconnects, probably because it tried to connect before the server is up
    stream::channel(100, move |mut output| async move {
        let mut state = State::Disconnected;

        loop {
            match &mut state {
                State::Disconnected => match connect_async(uri).await {
                    Ok((websocket, _)) => {
                        let (sender, receiver) = mpsc::channel(100);

                        let _ = output.send(Event::Connected(Connection(sender))).await;

                        state = State::Connected(websocket, receiver);
                    }
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        let _ = output.send(Event::Disconnected).await;
                    }
                },
                State::Connected(websocket, input) => {
                    let (write, read) = futures_util::StreamExt::split(websocket);
                    let fe = read.for_each(|message| {
                        let mut value = output.clone();
                        async move {
                            if let Ok(data) = message {
                                let data = data.to_text();
                                if let Ok(data) = data {
                                    let user: Result<UserExtended, serde_json::Error> =
                                        serde_json::from_str(data);
                                    if let Ok(user) = user {
                                        let _ = value
                                            .send(Event::MessageReceived(Message::User(user)))
                                            .await;
                                    }
                                }
                            }
                        }
                    });
                    fe.await;
                }
            }
        }
    })
}

pub enum State {
    Disconnected,
    Connected(
        WebSocketStream<MaybeTlsStream<TcpStream>>,
        Receiver<Message>,
    ),
}
#[derive(Debug, Clone)]
pub enum Event {
    Connected(Connection),
    Disconnected,
    MessageReceived(Message),
}
#[derive(Debug, Clone)]
pub enum Message {
    Connected,
    Disconnected,
    User(UserExtended),
    Tops(Vec<Score>),
    Firsts(Vec<Score>),
}
#[derive(Debug, Clone)]
pub struct Connection(mpsc::Sender<Message>);
