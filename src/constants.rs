/// TODO: Make port actually configured
pub const BASE_IP: &str = "127.0.0.1:7272";
pub const BASE_URI: &str = constcat::concat!("ws://", BASE_IP);
/// Endpoints
pub const USER_ENDPOINT: &str = "/";
pub const TOPS_ENDPOINT: &str = "/tops";
pub const FIRSTS_ENDPOINT: &str = "/firsts";
/// Full URIs
pub const USER_URI: &str = constcat::concat!(BASE_URI, USER_ENDPOINT);
pub const TOPS_URI: &str = constcat::concat!(BASE_URI, TOPS_ENDPOINT);
pub const FIRSTS_URI: &str = constcat::concat!(BASE_URI, FIRSTS_ENDPOINT);
