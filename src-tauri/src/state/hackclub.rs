use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct HackClubOAuthState {
    pub auth_result: Option<Value>,
}
lazy_static::lazy_static! {
    pub static ref HACKCLUB_STATE: Arc<Mutex<HackClubOAuthState>> = Arc::new(Mutex::new(HackClubOAuthState {
        auth_result: None,
    }));
}
