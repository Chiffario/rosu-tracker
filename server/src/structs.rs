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
    pub score_rank: Option<RespektiveUser>,
    // TODO: Add scorefarm api
}

impl TrackedData {
    pub fn new() -> Self {
        Self {
            user_extended: None,
            user_scores: None,
            user_firsts: None,
            user_recent: None,
            score_rank: None,
        }
    }
    #[allow(dead_code)]
    pub fn new_with(
        user_extended: impl Into<Option<UserExtended>>,
        user_scores: impl Into<Option<Vec<Score>>>,
        user_firsts: impl Into<Option<Vec<Score>>>,
        user_recent: impl Into<Option<Vec<Score>>>,
        score_rank: impl Into<Option<RespektiveUser>>,
    ) -> Self {
        Self {
            user_extended: user_extended.into(),
            user_scores: user_scores.into(),
            user_firsts: user_firsts.into(),
            user_recent: user_recent.into(),
            score_rank: score_rank.into(),
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

use std::{num::NonZeroU32, vec::IntoIter};

use serde::{Deserialize, Deserializer};

#[derive(Clone, Deserialize, Debug, PartialEq)]
pub struct RespektiveUserRankHighest {
    pub rank: u32,
    pub updated_at: String,
}
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct RespektiveUser {
    #[serde(deserialize_with = "zero_as_none")]
    pub rank: Option<NonZeroU32>,
    pub user_id: u32,
    pub rank_highest: Option<RespektiveUserRankHighest>,
    pub rank_history: Option<Vec<RankHistoryEntry>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct RankHistoryEntry {
    pub rank: Option<u32>,
    pub date: String,
}

pub struct RespektiveUsers {
    inner: IntoIter<RespektiveUser>,
}

impl From<Vec<RespektiveUser>> for RespektiveUsers {
    fn from(users: Vec<RespektiveUser>) -> Self {
        Self {
            inner: users.into_iter(),
        }
    }
}

impl Iterator for RespektiveUsers {
    type Item = Option<RespektiveUser>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|user| (user.rank.is_some() || user.rank_highest.is_some()).then_some(user))
    }
}

fn zero_as_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<NonZeroU32>, D::Error> {
    u32::deserialize(d).map(NonZeroU32::new)
}
