use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use mnemo_gateway::{Gateway, IncomingMessage, OutgoingMessage, QQGateway, WechatGateway};

use crate::config::{AppConfig, ChannelConfig};
use crate::db;
use crate::llm::{ChatMessage, LlmClient};

const CONTEXT_LIMIT: i64 = 20;

pub async fn run(config: AppConfig) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("mnemo=debug".parse()?))
        .init();

    if config.channels.is_empty() {
        anyhow::bail!("no channels configured, run `mnemo setup` first");
    }

    let mc = config
        .model
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("model not configured, run `mnemo setup` first"))?;

    info!("mnemo starting...");

    // Connect to PG if configured
    let pool = if let Some(ref pg_cfg) = config.pg {
        Some(Arc::new(db::connect(pg_cfg).await?))
    } else {
        warn!("PG not configured, running without memory");
        None
    };

    let llm = Arc::new(LlmClient::new(&mc.base_url, &mc.api_key, &mc.select_model));
    let (tx, mut rx) = mpsc::channel::<IncomingMessage>(64);

    // Start all configured channels and keep gateway references for sending
    let mut gateways: HashMap<String, Arc<dyn Gateway>> = HashMap::new();

    for channel in &config.channels {
        match channel {
            ChannelConfig::Wechat { token } => {
                let gateway = WechatGateway::new(Some(token.clone()));
                gateway.login().await?;
                let gw: Arc<dyn Gateway> = Arc::new(gateway);
                gateways.insert("wechat".into(), gw.clone());
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = gw.start(tx_clone).await {
                        error!(error = %e, "wechat gateway error");
                    }
                });
            }
            ChannelConfig::QQ(qq_config) => {
                let gateway = QQGateway::new(qq_config.clone());
                let gw: Arc<dyn Gateway> = Arc::new(gateway);
                gateways.insert("qq".into(), gw.clone());
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = gw.start(tx_clone).await {
                        error!(error = %e, "qq gateway error");
                    }
                });
            }
        }
    }

    drop(tx);

    // Message loop: receive → context → LLM → store → reply
    info!("waiting for messages...");
    while let Some(msg) = rx.recv().await {
        info!(from = %msg.sender_id, platform = %msg.platform, text = %msg.content, "received");

        let llm = llm.clone();
        let pool = pool.clone();
        let gw = gateways.get(&msg.platform).cloned();

        tokio::spawn(async move {
            let conversation_id = db::build_conversation_id(&msg);

            // Build LLM context
            let mut messages = Vec::new();

            if let Some(ref pool) = pool {
                match db::fetch_context(pool, &conversation_id, CONTEXT_LIMIT).await {
                    Ok(history) => {
                        for (role, content) in history {
                            messages.push(ChatMessage { role, content });
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to fetch context, continuing without history");
                    }
                }
            }

            // Append current user message
            messages.push(ChatMessage {
                role: "user".into(),
                content: msg.content.clone(),
            });

            // Query LLM
            let llm_resp = llm.query_with_context(messages).await;

            let (reply, metrics) = match llm_resp {
                Ok(r) => (r.content, r.metrics),
                Err(e) => {
                    error!(error = %e, "LLM query failed");
                    (format!("抱歉，处理出错了: {}", e), Default::default())
                }
            };

            info!(reply_len = reply.len(), "sending reply");

            // Store messages to DB
            if let Some(ref pool) = pool {
                let platform_msg_id = msg
                    .context
                    .get("msg_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Store user message
                if let Err(e) = db::store_message(
                    pool,
                    &conversation_id,
                    &msg.sender_id,
                    &msg.sender_name,
                    "",
                    "user",
                    &msg.content,
                    &msg.platform,
                    platform_msg_id.as_deref(),
                    "in",
                    &msg.context,
                )
                .await
                {
                    warn!(error = %e, "failed to store user message");
                }

                // Store assistant reply
                if let Err(e) = db::store_message(
                    pool,
                    &conversation_id,
                    "assistant",
                    "",
                    &msg.sender_id,
                    "assistant",
                    &reply,
                    &msg.platform,
                    None,
                    "out",
                    &serde_json::json!({}),
                )
                .await
                {
                    warn!(error = %e, "failed to store assistant message");
                }
            }

            // Send reply
            if let Some(ref gw) = gw {
                let out = OutgoingMessage {
                    session_id: msg.sender_id.clone(),
                    content: reply,
                    context: msg.context.clone(),
                };
                if let Err(e) = gw.send(out).await {
                    error!(error = %e, "failed to send reply");
                }

                // Send metrics as a follow-up message
                if metrics.elapsed_ms > 0 {
                    let metrics_text = format!(
                        "FTL: {}ms | Elapsed: {}ms | Tokens: {}→{}",
                        metrics.ftl_ms, metrics.elapsed_ms,
                        metrics.input_tokens, metrics.output_tokens,
                    );
                    let metrics_out = OutgoingMessage {
                        session_id: msg.sender_id.clone(),
                        content: metrics_text,
                        context: msg.context.clone(),
                    };
                    if let Err(e) = gw.send(metrics_out).await {
                        warn!(error = %e, "failed to send metrics");
                    }
                }
            }
        });
    }

    Ok(())
}
