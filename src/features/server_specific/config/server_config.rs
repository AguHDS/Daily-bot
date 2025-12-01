use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Feature {
    NicknameChanger,
    MentionResponse,
    Kick,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server_id: u64,
    pub general_channel_id: u64,
    pub enabled_features: Vec<Feature>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_id: 0,
            general_channel_id: 0,
            enabled_features: vec![
                Feature::NicknameChanger,
                Feature::MentionResponse,
                Feature::Kick,
            ],
        }
    }
}

impl ServerConfig {
    pub fn my_server() -> Self {
        Self {
            server_id: 1422605167580155914,
            general_channel_id: 1422605168662413456,
            enabled_features: vec![
                Feature::NicknameChanger,
                Feature::MentionResponse,
                Feature::Kick,
            ],
        }
    }
}
