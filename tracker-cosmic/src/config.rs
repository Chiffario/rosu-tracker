use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, ConfigGet, CosmicConfigEntry};
use serde::{Deserialize, Serialize};
use color_eyre::eyre::Result;
use types::Api;

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

pub fn get_config_cosmic() -> Result<Api> {
    let config_handler =
        cosmic_config::Config::new(constants::APP_ID, constants::CONFIG_VERSION)?;
    Ok(Api {
        id: config_handler.get::<String>("user_client")?,
        secret: config_handler.get::<String>("user_secret")?,
        username: config_handler.get::<String>("tracked_user_name")?,
    })
}