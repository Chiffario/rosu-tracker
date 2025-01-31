use crate::setup::{set_cosmic_config, Api};
use ::cosmic::{
    app::{run, Settings},
    iced::{self, Limits},
};
use app::AppModel;
pub mod app;

mod socket;
/// Initialise the gui with its local runtime
// pub fn init() -> iced::Result {
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
