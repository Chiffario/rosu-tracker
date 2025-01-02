use ::cosmic::{
    app::{run, Settings},
    iced::{self, Limits},
};
use app::AppModel;
pub mod app;
pub mod config;
mod socket;
pub fn init() -> iced::Result {
    let settings = Settings::default().size_limits(Limits::NONE.min_width(360.0).min_height(180.0));
    run::<AppModel>(settings, ())
}
