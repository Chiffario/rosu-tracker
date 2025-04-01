use app::AppModel;
use cosmic::{app::{run, Settings}, cosmic_config, iced::{self, Limits}};
use types::Api;

pub mod app;
mod components;
mod socket;
mod config;
mod image_fetch;

/// Initialise the gui with its local runtime
// pub fn init() -> cosmic::Result {
//     let settings = Settings::default().size_limits(Limits::NONE.min_width(360.0).min_height(180.0));
//     run::<AppModel>(settings, ())
// }

pub fn init_with_flags(config: Option<Api>) -> iced::Result {
    let settings = Settings::default().size_limits(Limits::NONE.min_width(360.0).min_height(180.0));
    if let Some(init_flags) = config {
        set_cosmic_config(init_flags);
    }
    run::<AppModel>(settings, ())
}

pub fn set_cosmic_config(new_config: Api) -> () {
    let config_handler =
        cosmic_config::Config::new(constants::APP_ID, constants::CONFIG_VERSION)
            .unwrap();
    let mut config = crate::config::Config::default();
    let _ = config.set_user_client(&config_handler, new_config.id);
    let _ = config.set_user_secret(&config_handler, new_config.secret);
    let _ = config.set_tracked_user_name(&config_handler, new_config.username);
}