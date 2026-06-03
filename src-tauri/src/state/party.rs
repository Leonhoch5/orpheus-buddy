use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct PartyState {
    pub last_known_seconds: u32,
    pub last_party_threshold: u32,
}

lazy_static::lazy_static! {
    pub static ref PARTY_STATE: Arc<Mutex<PartyState>> = Arc::new(Mutex::new(PartyState {
        last_known_seconds: 0,
        last_party_threshold: 0,
    }));
}