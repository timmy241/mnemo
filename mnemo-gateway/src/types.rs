use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub session_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub content: String,
    pub platform: String,
    pub timestamp: i64,
    /// Platform-specific context for replying (e.g. WeChat context_token).
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub session_id: String,
    pub content: String,
}
