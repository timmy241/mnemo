use serde::{Deserialize, Serialize};

pub const DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
pub const BOT_TYPE: &str = "3";
pub const CHANNEL_VERSION: &str = "1.0.2";

// ── Login ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrcodeResponse {
    pub qrcode: String,
    pub qrcode_img_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrcodeStatusResponse {
    pub status: String,
    pub bot_token: Option<String>,
    pub baseurl: Option<String>,
    pub ilink_bot_id: Option<String>,
    pub ilink_user_id: Option<String>,
}

// ── Token session ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSession {
    pub token: String,
    pub base_url: String,
    pub bot_id: String,
    pub user_id: String,
    pub saved_at: String,
}

// ── Messages ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUpdatesRequest {
    pub get_updates_buf: String,
    pub base_info: BaseInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseInfo {
    pub channel_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetUpdatesResponse {
    #[serde(default)]
    pub msgs: Vec<WeixinMessage>,
    #[serde(default)]
    pub get_updates_buf: Option<String>,
    #[serde(default)]
    pub sync_buf: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeixinMessage {
    #[serde(default)]
    pub seq: Option<i64>,
    #[serde(default)]
    pub message_id: Option<i64>,
    pub from_user_id: String,
    pub to_user_id: String,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub create_time_ms: Option<i64>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub message_type: i32,
    #[serde(default)]
    pub message_state: Option<i32>,
    #[serde(default)]
    pub context_token: Option<String>,
    #[serde(default)]
    pub item_list: Option<Vec<MessageItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageItem {
    #[serde(rename = "type")]
    pub item_type: i32,
    #[serde(default)]
    pub create_time_ms: Option<i64>,
    #[serde(default)]
    pub is_completed: Option<bool>,
    #[serde(default)]
    pub text_item: Option<TextItem>,
    #[serde(default)]
    pub voice_item: Option<VoiceItem>,
    #[serde(default)]
    pub file_item: Option<FileItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextItem {
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceItem {
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub file_name: Option<String>,
}

// ── Send message ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub msg: OutgoingMsg,
    pub base_info: BaseInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMsg {
    pub from_user_id: String,
    pub to_user_id: String,
    pub client_id: String,
    pub message_type: i32,
    pub message_state: i32,
    pub context_token: String,
    pub item_list: Vec<MessageItem>,
}
