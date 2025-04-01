use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Api {
    pub id: String,
    pub secret: String,
    pub username: String,
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}
