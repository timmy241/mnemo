use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};

use crate::error::GatewayError;
use crate::gateway::Gateway;
use crate::types::{IncomingMessage, OutgoingMessage};
use crate::wechat::client::WechatClient;
use crate::wechat::types::TokenSession;

pub struct WechatGateway {
    client: Arc<Mutex<WechatClient>>,
    saved_token: Option<TokenSession>,
    session: Arc<Mutex<Option<TokenSession>>>,
}

impl WechatGateway {
    /// Create gateway with optional saved token from config.
    pub fn new(saved_token: Option<TokenSession>) -> Self {
        let client = if let Some(ref s) = saved_token {
            WechatClient::new(&s.base_url).with_token(&s.token)
        } else {
            WechatClient::new(crate::wechat::types::DEFAULT_BASE_URL)
        };
        Self {
            client: Arc::new(Mutex::new(client)),
            saved_token,
            session: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the current session after login (useful for saving token back to config).
    pub async fn session(&self) -> Option<TokenSession> {
        self.session.lock().await.clone()
    }

    /// Login: use saved token or do interactive QR-code scan.
    /// Call this before `start()` so the session is available for saving.
    pub async fn login(&self) -> Result<TokenSession, GatewayError> {
        if let Some(ref session) = self.saved_token {
            info!(bot_id = %session.bot_id, "using saved token");
            *self.session.lock().await = Some(session.clone());
            return Ok(session.clone());
        }

        self.interactive_login().await
    }

    /// Interactive QR-code login flow.
    async fn interactive_login(&self) -> Result<TokenSession, GatewayError> {
        let client = WechatClient::new(crate::wechat::types::DEFAULT_BASE_URL);

        info!("requesting login QR code...");
        let qr = client.get_qrcode().await?;
        println!("\n=== 微信扫码登录 ===");
        println!("请用微信扫描以下链接对应的二维码：");
        println!("{}\n", qr.qrcode_img_content);
        println!("等待扫码确认...\n");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
        let mut current_qr = qr.qrcode.clone();

        loop {
            if std::time::Instant::now() > deadline {
                return Err(GatewayError::Connection("login timed out".into()));
            }

            let status = client.get_qrcode_status(&current_qr).await?;
            match status.status.as_str() {
                "wait" => {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                "scaned" => {
                    info!("QR code scanned, waiting for confirmation...");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                "expired" => {
                    warn!("QR code expired, refreshing...");
                    let new_qr = client.get_qrcode().await?;
                    current_qr = new_qr.qrcode;
                    println!("二维码已过期，请重新扫描：");
                    println!("{}\n", new_qr.qrcode_img_content);
                }
                "confirmed" => {
                    let token = status
                        .bot_token
                        .ok_or_else(|| GatewayError::Connection("no token in response".into()))?;
                    let base_url = status
                        .baseurl
                        .unwrap_or_else(|| crate::wechat::types::DEFAULT_BASE_URL.to_string());
                    let bot_id = status.ilink_bot_id.unwrap_or_default();
                    let user_id = status.ilink_user_id.unwrap_or_default();

                    let session = TokenSession {
                        token: token.clone(),
                        base_url: base_url.clone(),
                        bot_id: bot_id.clone(),
                        user_id,
                        saved_at: chrono::Utc::now().to_rfc3339(),
                    };

                    info!(bot_id = %bot_id, "login successful");

                    *self.client.lock().await =
                        WechatClient::new(&base_url).with_token(&token);
                    *self.session.lock().await = Some(session.clone());

                    return Ok(session);
                }
                other => {
                    warn!(status = other, "unknown QR status");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Extract plain text from a WeixinMessage.
    fn extract_text(msg: &crate::wechat::types::WeixinMessage) -> String {
        for item in msg.item_list.as_deref().unwrap_or_default() {
            match item.item_type {
                1 => {
                    if let Some(ref t) = item.text_item {
                        if let Some(ref text) = t.text {
                            return text.clone();
                        }
                    }
                }
                2 => return "[图片]".into(),
                3 => {
                    if let Some(ref v) = item.voice_item {
                        if let Some(ref text) = v.text {
                            return format!("[语音] {}", text);
                        }
                    }
                    return "[语音]".into();
                }
                4 => {
                    if let Some(ref f) = item.file_item {
                        return format!(
                            "[文件] {}",
                            f.file_name.as_deref().unwrap_or("unknown")
                        );
                    }
                    return "[文件]".into();
                }
                5 => return "[视频]".into(),
                _ => {}
            }
        }
        "[空消息]".into()
    }
}

#[async_trait]
impl Gateway for WechatGateway {
    async fn start(&self, tx: mpsc::Sender<IncomingMessage>) -> Result<(), GatewayError> {
        let session = self.session.lock().await.clone()
            .ok_or_else(|| GatewayError::Connection("not logged in, call login() first".into()))?;
        let client = self.client.lock().await.clone();

        info!(bot_id = %session.bot_id, "starting message polling");

        let mut buf = String::new();
        loop {
            match client.get_updates(&buf).await {
                Ok(resp) => {
                    if let Some(new_buf) = resp.get_updates_buf {
                        buf = new_buf;
                    }

                    for msg in resp.msgs {
                        if msg.message_type != 1 {
                            continue;
                        }

                        let text = Self::extract_text(&msg);
                        let sender_id = msg.from_user_id.clone();

                        info!(from = %sender_id, text = %text, "received message");

                        let ctx = serde_json::json!({
                            "context_token": msg.context_token,
                        });
                        let incoming = IncomingMessage {
                            session_id: sender_id.clone(),
                            sender_id: sender_id.clone(),
                            sender_name: String::new(),
                            content: text,
                            platform: "wechat".into(),
                            timestamp: chrono::Utc::now().timestamp(),
                            context: ctx,
                        };

                        if tx.send(incoming).await.is_err() {
                            warn!("receiver dropped, stopping poll");
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("session timeout") || msg.contains("-14") {
                        error!("session expired, need re-login");
                        return Err(GatewayError::Connection("session expired".into()));
                    }
                    warn!(error = %msg, "poll error, retrying in 3s");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    }

    async fn send(&self, message: OutgoingMessage) -> Result<(), GatewayError> {
        let client = self.client.lock().await;
        client
            .send_message(&message.session_id, &message.content, "")
            .await
    }

    async fn stop(&self) -> Result<(), GatewayError> {
        info!("gateway stopped");
        Ok(())
    }
}

impl WechatGateway {
    /// Send a reply with context_token for proper conversation threading.
    pub async fn send_reply(
        &self,
        to: &str,
        text: &str,
        context_token: &str,
    ) -> Result<(), GatewayError> {
        let client = self.client.lock().await;
        client.send_message(to, text, context_token).await
    }
}
