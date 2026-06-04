use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SlackOAuthState {
    pub auth_result: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct SlackNotificationState {
    pub last_dm_ts: String,
    pub last_mention_ts: String,
    pub user_id: Option<String>,
}

lazy_static::lazy_static! {
    pub static ref SLACK_STATE: Arc<Mutex<SlackOAuthState>> = Arc::new(Mutex::new(SlackOAuthState {
        auth_result: None,
    }));
    pub static ref SLACK_NOTIF_STATE: Arc<Mutex<SlackNotificationState>> = Arc::new(Mutex::new(SlackNotificationState {
        last_dm_ts: "0".to_string(),
        last_mention_ts: "0".to_string(),
        user_id: None,
    }));
}
