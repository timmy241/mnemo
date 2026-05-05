use std::sync::Arc;

use async_trait::async_trait;
use botrs::models::gateway::Ready;
use botrs::models::message::{C2CMessageParams, DirectMessageParams, GroupMessageParams, MessageParams};
use botrs::{BotApi, Client, Context, EventHandler, Intents, Token};
use tokio::sync::{mpsc, oneshot, OnceCell};
use tracing::{error, info, warn};

use crate::error::GatewayError;
use crate::gateway::Gateway;
use crate::qq::types::QQChannelConfig;
use crate::types::{IncomingMessage, OutgoingMessage};

/// Shared state between the event handler and the gateway.
struct SharedState {
    api: OnceCell<(Arc<BotApi>, Token)>,
}

struct QQBotHandler {
    tx: mpsc::Sender<IncomingMessage>,
    ready_tx: std::sync::Mutex<Option<oneshot::Sender<()>>>,
    state: Arc<SharedState>,
}

#[async_trait]
impl EventHandler for QQBotHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(user = %ready.user.username, "QQ bot connected");
        let _ = self.state.api.set((ctx.api.clone(), ctx.token.clone()));
        if let Some(tx) = self.ready_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    async fn message_create(&self, _ctx: Context, message: botrs::Message) {
        if message.is_from_bot() {
            return;
        }
        let (content, sender_id) = match (&message.content, &message.author) {
            (Some(c), Some(a)) => {
                let id = a.id.as_ref().map(|s| s.to_string()).unwrap_or_default();
                (c.clone(), id)
            }
            _ => return,
        };
        let channel_id = message.channel_id.as_ref().map(|s| s.to_string()).unwrap_or_default();
        if content.is_empty() {
            return;
        }
        info!(from = %sender_id, channel = %channel_id, "QQ guild message");

        let mut ctx_map = serde_json::Map::new();
        ctx_map.insert("msg_type".into(), serde_json::json!("guild"));
        if let Some(id) = &message.id {
            ctx_map.insert("msg_id".into(), serde_json::json!(id.to_string()));
        }
        ctx_map.insert("channel_id".into(), serde_json::json!(channel_id));

        let incoming = IncomingMessage {
            session_id: sender_id.clone(),
            sender_id,
            sender_name: message.author.as_ref().and_then(|a| a.username.clone()).unwrap_or_default(),
            content,
            platform: "qq".into(),
            timestamp: chrono::Utc::now().timestamp(),
            context: serde_json::Value::Object(ctx_map),
        };

        if self.tx.send(incoming).await.is_err() {
            warn!("receiver dropped");
        }
    }

    async fn group_message_create(&self, _ctx: Context, message: botrs::GroupMessage) {
        let (content, sender_id) = match (&message.content, &message.author) {
            (Some(c), Some(a)) => {
                let id = a.member_openid.clone().unwrap_or_default();
                (c.clone(), id)
            }
            _ => return,
        };
        let group_openid = message.group_openid.clone().unwrap_or_default();
        if content.is_empty() {
            return;
        }
        info!(from = %sender_id, group = %group_openid, "QQ group message");

        let mut ctx_map = serde_json::Map::new();
        ctx_map.insert("msg_type".into(), serde_json::json!("group"));
        if let Some(id) = &message.id {
            ctx_map.insert("msg_id".into(), serde_json::json!(id));
        }
        ctx_map.insert("group_openid".into(), serde_json::json!(group_openid));

        let incoming = IncomingMessage {
            session_id: sender_id.clone(),
            sender_id,
            sender_name: String::new(),
            content,
            platform: "qq".into(),
            timestamp: chrono::Utc::now().timestamp(),
            context: serde_json::Value::Object(ctx_map),
        };

        if self.tx.send(incoming).await.is_err() {
            warn!("receiver dropped");
        }
    }

    async fn c2c_message_create(&self, _ctx: Context, message: botrs::C2CMessage) {
        let (content, sender_id) = match (&message.content, &message.author) {
            (Some(c), Some(a)) => {
                let id = a.user_openid.clone().unwrap_or_default();
                (c.clone(), id)
            }
            _ => return,
        };
        if content.is_empty() {
            return;
        }
        info!(from = %sender_id, "QQ C2C message");

        let mut ctx_map = serde_json::Map::new();
        ctx_map.insert("msg_type".into(), serde_json::json!("c2c"));
        if let Some(id) = &message.id {
            ctx_map.insert("msg_id".into(), serde_json::json!(id));
        }

        let incoming = IncomingMessage {
            session_id: sender_id.clone(),
            sender_id,
            sender_name: String::new(),
            content,
            platform: "qq".into(),
            timestamp: chrono::Utc::now().timestamp(),
            context: serde_json::Value::Object(ctx_map),
        };

        if self.tx.send(incoming).await.is_err() {
            warn!("receiver dropped");
        }
    }

    async fn direct_message_create(&self, _ctx: Context, message: botrs::DirectMessage) {
        let content = match &message.content {
            Some(c) if !c.is_empty() => c.clone(),
            _ => return,
        };
        let guild_id = message.guild_id.as_ref().map(|s| s.to_string()).unwrap_or_default();
        info!(guild = %guild_id, "QQ direct message");

        let mut ctx_map = serde_json::Map::new();
        ctx_map.insert("msg_type".into(), serde_json::json!("dm"));
        if let Some(id) = &message.id {
            ctx_map.insert("msg_id".into(), serde_json::json!(id.to_string()));
        }
        ctx_map.insert("guild_id".into(), serde_json::json!(guild_id));

        let incoming = IncomingMessage {
            session_id: "dm".into(),
            sender_id: "dm".into(),
            sender_name: String::new(),
            content,
            platform: "qq".into(),
            timestamp: chrono::Utc::now().timestamp(),
            context: serde_json::Value::Object(ctx_map),
        };

        if self.tx.send(incoming).await.is_err() {
            warn!("receiver dropped");
        }
    }
}

pub struct QQGateway {
    config: QQChannelConfig,
    state: Arc<SharedState>,
}

impl QQGateway {
    pub fn new(config: QQChannelConfig) -> Self {
        Self {
            config,
            state: Arc::new(SharedState {
                api: OnceCell::new(),
            }),
        }
    }

    pub async fn send_reply(
        &self,
        msg_type: &str,
        msg_id: &str,
        text: &str,
        target_id: &str,
    ) -> Result<(), GatewayError> {
        let (api, token) = self
            .state
            .api
            .get()
            .ok_or_else(|| GatewayError::Message("QQ bot not connected".into()))?;

        match msg_type {
            "guild" => {
                let params = MessageParams::new_text(text).with_reply(msg_id);
                api.post_message_with_params(token, target_id, params)
                    .await
                    .map_err(|e| GatewayError::Message(e.to_string()))?;
            }
            "group" => {
                let params = GroupMessageParams::new_text(text).with_reply(msg_id);
                api.post_group_message_with_params(token, target_id, params)
                    .await
                    .map_err(|e| GatewayError::Message(e.to_string()))?;
            }
            "c2c" => {
                let params = C2CMessageParams::new_text(text);
                api.post_c2c_message_with_params(token, target_id, params)
                    .await
                    .map_err(|e| GatewayError::Message(e.to_string()))?;
            }
            "dm" => {
                let params = DirectMessageParams::new_text(text).with_reply(msg_id);
                api.post_dms_with_params(token, target_id, params)
                    .await
                    .map_err(|e| GatewayError::Message(e.to_string()))?;
            }
            _ => {
                warn!(msg_type = msg_type, "unknown QQ message type");
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Gateway for QQGateway {
    async fn start(&self, tx: mpsc::Sender<IncomingMessage>) -> Result<(), GatewayError> {
        let (ready_tx, ready_rx) = oneshot::channel();

        let handler = QQBotHandler {
            tx,
            ready_tx: std::sync::Mutex::new(Some(ready_tx)),
            state: self.state.clone(),
        };

        let token = Token::new(&self.config.app_id, &self.config.secret);
        let intents = Intents::default()
            .with_guild_messages()
            .with_public_messages()
            .with_direct_message();

        info!("starting QQ bot...");

        let handle = tokio::spawn(async move {
            let mut client = Client::new(token, intents, handler, false).unwrap();
            if let Err(e) = client.start().await {
                error!(error = %e, "QQ bot error");
            }
        });

        // Wait for ready signal
        match tokio::time::timeout(std::time::Duration::from_secs(30), ready_rx).await {
            Ok(Ok(())) => info!("QQ bot ready"),
            Ok(Err(_)) => {
                return Err(GatewayError::Connection(
                    "QQ bot failed to initialize".into(),
                ));
            }
            Err(_) => {
                return Err(GatewayError::Connection(
                    "QQ bot connection timed out".into(),
                ));
            }
        }

        // Wait for the bot task to finish
        let _ = handle.await;
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> Result<(), GatewayError> {
        let msg_type = message
            .context
            .get("msg_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let msg_id = message
            .context
            .get("msg_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let target_id = &message.session_id;

        self.send_reply(msg_type, msg_id, &message.content, target_id)
            .await
    }

    async fn stop(&self) -> Result<(), GatewayError> {
        info!("QQ gateway stopped");
        Ok(())
    }
}
