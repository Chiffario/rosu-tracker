/// TODO: Make port actually configured
pub const BASE_IP: &str = "127.0.0.1:7272";
#[cfg(feature = "gui")]
pub const BASE_URI: &str = constcat::concat!("ws://", BASE_IP);
/// Endpoints
pub const USER_ENDPOINT: &str = "/";
pub const TOPS_ENDPOINT: &str = "/tops";
pub const FIRSTS_ENDPOINT: &str = "/firsts";
pub const RECENT_ENDPOINT: &str = "/recent";
/// Full URIs
#[cfg(feature = "gui")]
pub const USER_URI: &str = constcat::concat!(BASE_URI, USER_ENDPOINT);
#[cfg(feature = "gui")]
pub const TOPS_URI: &str = constcat::concat!(BASE_URI, TOPS_ENDPOINT);
#[cfg(feature = "gui")]
pub const FIRSTS_URI: &str = constcat::concat!(BASE_URI, FIRSTS_ENDPOINT);
#[cfg(feature = "gui")]
pub const RECENT_URI: &str = constcat::concat!(BASE_URI, RECENT_ENDPOINT);

pub const CONFIG_VERSION: u64 = 1;
pub(crate) const APP_ID: &'static str = "com.chiffa.rosuTracker";
