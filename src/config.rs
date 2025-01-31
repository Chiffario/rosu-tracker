use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq, Deserialize, Serialize)]
#[version = 1]
pub struct Config {
    #[serde(default)]
    user_client: String,
    #[serde(default)]
    user_secret: String,
    #[serde(default)]
    tracked_user_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user_client: String::new(),
            user_secret: String::new(),
            tracked_user_name: String::new(),
        }
    }
}
