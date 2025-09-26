use std::sync::{Arc, RwLock};

pub type ClientConfigShared = Arc<RwLock<ClientConfig>>;

#[derive(Debug)]
pub struct ClientConfig {
    pub volume: f32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self { volume: 0.5 }
    }
}

impl ClientConfig {
    pub fn new_shared() -> ClientConfigShared {
        Arc::new(RwLock::new(ClientConfig::default()))
    }
}
