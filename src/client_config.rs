use std::sync::{Arc, RwLock};

pub type ClientConfigShared = Arc<RwLock<ClientConfig>>;

#[derive(Debug, Default)]
pub struct ClientConfig {
    pub volume: f32,
}

impl ClientConfig {
    pub fn new_shared() -> ClientConfigShared {
        Arc::new(RwLock::new(ClientConfig::default()))
    }
}
