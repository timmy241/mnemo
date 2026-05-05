use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QQChannelConfig {
    pub app_id: String,
    pub secret: String,
}
