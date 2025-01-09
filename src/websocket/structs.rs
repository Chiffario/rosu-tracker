use std::sync::Arc;

use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use rosu_v2::prelude::{Score, UserExtended};
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
pub enum WsKind {
    User,
    Tops,
    Firsts,
    Recent,
}
#[derive(Debug)]
pub struct WsClient {
    pub kind: WsKind,
    pub client: WebSocketStream<TokioIo<Upgraded>>,
}

pub type Arm<T> = Arc<Mutex<T>>;
// pub type ArRwlock<T> = Arc<RwLock<T>>;
pub type Clients = Arm<Vec<WsClient>>;
#[derive(Clone, Debug, PartialEq)]
pub struct TrackedData {
    pub user_extended: Option<UserExtended>,
    pub user_scores: Option<Vec<Score>>,
    pub user_firsts: Option<Vec<Score>>,
    pub user_recent: Option<Vec<Score>>,
    // TODO: Add scorefarm api
}

impl TrackedData {
    pub fn new() -> Self {
        Self {
            user_extended: None,
            user_scores: None,
            user_firsts: None,
            user_recent: None,
        }
    }
    #[allow(dead_code)]
    pub fn new_with(
        user_extended: impl Into<Option<UserExtended>>,
        user_scores: impl Into<Option<Vec<Score>>>,
        user_firsts: impl Into<Option<Vec<Score>>>,
        user_recent: impl Into<Option<Vec<Score>>>,
    ) -> Self {
        Self {
            user_extended: user_extended.into(),
            user_scores: user_scores.into(),
            user_firsts: user_firsts.into(),
            user_recent: user_recent.into(),
        }
    }
    pub fn insert(
        &mut self,
        user_extended: impl Into<Option<UserExtended>>,
        user_scores: impl Into<Option<Vec<Score>>>,
        user_firsts: impl Into<Option<Vec<Score>>>,
        user_recent: impl Into<Option<Vec<Score>>>,
    ) {
        self.user_extended = user_extended.into();
        self.user_scores = user_scores.into();
        self.user_firsts = user_firsts.into();
        self.user_recent = user_recent.into();
    }
}
