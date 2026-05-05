use base64::Engine;
use rand::Rng;
use reqwest::Client;
use tracing::{debug, info, warn};

use crate::error::GatewayError;
use crate::wechat::types::*;

#[derive(Clone)]
pub struct WechatClient {
    http: Client,
    base_url: String,
    token: Option<String>,
}

impl WechatClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(45))
                .build()
                .expect("failed to build HTTP client"),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    // ── Auth helpers ───────────────────────────────────────────────────────────

    /// X-WECHAT-UIN: random uint32 → decimal string → base64
    fn random_uin() -> String {
        let val: u32 = rand::rng().random();
        base64::engine::general_purpose::STANDARD.encode(val.to_string().as_bytes())
    }

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("AuthorizationType", "ilink_bot_token".parse().unwrap());
        headers.insert("X-WECHAT-UIN", Self::random_uin().parse().unwrap());
        if let Some(ref token) = self.token {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    // ── Login ──────────────────────────────────────────────────────────────────

    pub async fn get_qrcode(&self) -> Result<QrcodeResponse, GatewayError> {
        let url = format!(
            "{}/ilink/bot/get_bot_qrcode?bot_type={}",
            self.base_url, BOT_TYPE
        );
        info!(url = %url, "GET get_bot_qrcode");
        let resp = self.http.get(&url).send().await?;
        info!(status = %resp.status(), "get_bot_qrcode response");
        let text = resp.text().await?;
        debug!(body = %text, "get_bot_qrcode body");
        let data: QrcodeResponse = serde_json::from_str(&text)?;
        Ok(data)
    }

    pub async fn get_qrcode_status(
        &self,
        qrcode: &str,
    ) -> Result<QrcodeStatusResponse, GatewayError> {
        let url = format!(
            "{}/ilink/bot/get_qrcode_status?qrcode={}",
            self.base_url, qrcode
        );
        info!(url = %url, "GET get_qrcode_status");
        let resp = self.http.get(&url).send().await?;
        info!(status = %resp.status(), "get_qrcode_status response");
        let text = resp.text().await?;
        debug!(body = %text, "get_qrcode_status body");
        let data: QrcodeStatusResponse = serde_json::from_str(&text)?;
        Ok(data)
    }

    // ── Messaging ──────────────────────────────────────────────────────────────

    pub async fn get_updates(
        &self,
        buf: &str,
    ) -> Result<GetUpdatesResponse, GatewayError> {
        let url = format!("{}/ilink/bot/getupdates", self.base_url);
        let body = GetUpdatesRequest {
            get_updates_buf: buf.to_string(),
            base_info: BaseInfo {
                channel_version: CHANNEL_VERSION.to_string(),
            },
        };
        info!(url = %url, buf_len = buf.len(), "POST getupdates");
        let resp = self
            .http
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .timeout(std::time::Duration::from_secs(38))
            .send()
            .await?;

        let status = resp.status();
        info!(status = %status, "getupdates response");

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            warn!(body = %text, "getupdates non-success");
            return Err(GatewayError::Connection(format!(
                "getupdates HTTP {}: {}",
                status, text
            )));
        }

        let text = resp.text().await?;
        if text.is_empty() {
            info!("getupdates empty body, returning empty msgs");
            return Ok(empty_response(buf));
        }
        debug!(body_len = text.len(), "getupdates body");

        match serde_json::from_str::<GetUpdatesResponse>(&text) {
            Ok(data) => {
                info!(msg_count = data.msgs.len(), "getupdates parsed ok");
                Ok(data)
            }
            Err(e) => {
                warn!(error = %e, body = %text, "getupdates json parse failed, returning empty");
                Ok(empty_response(buf))
            }
        }
    }

    pub async fn send_message(
        &self,
        to_user_id: &str,
        text: &str,
        context_token: &str,
    ) -> Result<(), GatewayError> {
        let url = format!("{}/ilink/bot/sendmessage", self.base_url);
        let body = SendMessageRequest {
            msg: OutgoingMsg {
                from_user_id: String::new(),
                to_user_id: to_user_id.to_string(),
                client_id: format!("mnemo-{}", uuid_simple()),
                message_type: 2,   // BOT
                message_state: 2,  // FINISH
                context_token: context_token.to_string(),
                item_list: vec![MessageItem {
                    item_type: 1, // TEXT
                    create_time_ms: None,
                    is_completed: None,
                    text_item: Some(TextItem {
                        text: Some(text.to_string()),
                    }),
                    voice_item: None,
                    file_item: None,
                }],
            },
            base_info: BaseInfo {
                channel_version: CHANNEL_VERSION.to_string(),
            },
        };

        info!(url = %url, to = %to_user_id, "POST sendmessage");
        let resp = self
            .http
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        info!(status = %status, "sendmessage response");

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            warn!(body = %text, "sendmessage non-success");
            return Err(GatewayError::Message(format!(
                "sendmessage HTTP {}: {}",
                status, text
            )));
        }

        debug!(to = to_user_id, "message sent");
        Ok(())
    }
}

fn empty_response(buf: &str) -> GetUpdatesResponse {
    GetUpdatesResponse {
        msgs: vec![],
        get_updates_buf: Some(buf.to_string()),
        sync_buf: None,
    }
}

fn uuid_simple() -> String {
    let mut rng = rand::rng();
    let id: u64 = rng.random();
    format!("{:016x}", id)
}
