use cosmic::iced::{
    futures::{
        channel::mpsc::{self},
        SinkExt,
    },
    stream,
};
use futures_util::{Stream, StreamExt};
use rosu_v2::prelude::{Score, UserExtended};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use crate::constants::{FIRSTS_URI, RECENT_URI, TOPS_URI, USER_URI};

/// Create a stream of `UserExtended` messages
pub fn connect_user() -> impl Stream<Item = Event> {
    connect_websocket::<UserExtended, User>(USER_URI)
}
/// Create a stream of user first place scores
pub fn connect_firsts() -> impl Stream<Item = Event> {
    connect_websocket::<Vec<Score>, Firsts>(FIRSTS_URI)
}
/// Create a stream of user top scores
pub fn connect_tops() -> impl Stream<Item = Event> {
    connect_websocket::<Vec<Score>, Tops>(TOPS_URI)
}
pub fn connect_recent() -> impl Stream<Item = Event> {
    connect_websocket::<Vec<Score>, Recent>(RECENT_URI)
}
/// General websocket stream creation. `U` should be a newtype over `T`
fn connect_websocket<T, U>(uri: &str) -> impl Stream<Item = Event> + use<'_, T, U>
where
    T: for<'a> Deserialize<'a>,
    U: IntoMessage<T>,
{
    stream::channel(100, move |mut output| async move {
        let mut state = State::Disconnected;

        loop {
            match &mut state {
                State::Disconnected => match connect_async(uri).await {
                    Ok((websocket, _)) => {
                        let (sender, _) = mpsc::channel(100);

                        let _ = output.send(Event::Connected(Connection(sender))).await;

                        state = State::Connected(websocket);
                    }
                    Err(_) => {
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                        let _ = output.send(Event::Disconnected).await;
                    }
                },
                State::Connected(websocket) => {
                    let (_, read) = futures_util::StreamExt::split(websocket);
                    let fe = read.for_each(|x| {
                        let mut value = output.clone();
                        async move {
                            if let Ok(data) = x {
                                let data = data.to_text();
                                if let Ok(data) = data {
                                    let user: Result<T, serde_json::Error> =
                                        serde_json::from_str(data);
                                    if let Ok(user) = user {
                                        let _ = value
                                            .send(Event::MessageReceived(U::into_message(user)))
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
    Connected(WebSocketStream<MaybeTlsStream<TcpStream>>),
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
    User(Box<UserExtended>),
    Tops(Vec<Score>),
    Firsts(Vec<Score>),
    Recent(Vec<Score>),
}
/// Workaround to allow type-level difference between user tops and user firsts
trait IntoMessage<T> {
    fn into_message(value: T) -> Message;
}

impl IntoMessage<UserExtended> for User {
    fn into_message(value: UserExtended) -> Message {
        Message::User(Box::new(value))
    }
}

impl IntoMessage<Vec<Score>> for Tops {
    fn into_message(value: Vec<Score>) -> Message {
        Message::Tops(value)
    }
}

impl IntoMessage<Vec<Score>> for Firsts {
    fn into_message(value: Vec<Score>) -> Message {
        Message::Firsts(value)
    }
}

impl IntoMessage<Vec<Score>> for Recent {
    fn into_message(value: Vec<Score>) -> Message {
        Message::Recent(value)
    }
}

struct User(UserExtended);

impl From<User> for UserExtended {
    fn from(val: User) -> Self {
        val.0
    }
}

struct Tops(Vec<Score>);

impl From<Tops> for Vec<Score> {
    fn from(val: Tops) -> Self {
        val.0
    }
}
struct Firsts(Vec<Score>);

impl From<Firsts> for Vec<Score> {
    fn from(val: Firsts) -> Self {
        val.0
    }
}

struct Recent(Vec<Score>);

impl From<Recent> for Vec<Score> {
    fn from(val: Recent) -> Self {
        val.0
    }
}
#[allow(dead_code, reason = "WIP")]
#[derive(Debug, Clone)]
pub struct Connection(mpsc::Sender<Message>);
