use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use mnemo_gateway::{Gateway, WechatGateway};

use crate::config::AppConfig;
use crate::llm::LlmClient;

pub async fn run(config: AppConfig) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("mnemo=debug".parse()?))
        .init();

    // Require wechat_token
    let _token = config
        .wechat_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("wechat_token not configured, run `mnemo setup` first"))?;

    // Require model config
    let mc = config
        .model
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("model not configured, run `mnemo setup` first"))?;

    info!("mnemo starting...");

    // Login WeChat (reuse saved token)
    let gateway = WechatGateway::new(config.wechat_token.clone());
    gateway.login().await?;

    // Build LLM client
    let llm = LlmClient::new(&mc.base_url, &mc.api_key, &mc.select_model);

    // Start gateway polling
    let gw = Arc::new(gateway);
    let (tx, mut rx) = mpsc::channel::<mnemo_gateway::IncomingMessage>(64);
    let gw_clone = gw.clone();
    let poll_handle = tokio::spawn(async move {
        if let Err(e) = gw_clone.start(tx).await {
            error!(error = %e, "gateway error");
        }
    });

    // Message loop: receive → LLM → reply
    info!("waiting for messages...");
    while let Some(msg) = rx.recv().await {
        info!(from = %msg.sender_id, text = %msg.content, "received");

        let context_token = msg
            .context
            .get("context_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        match llm.query(&msg.content).await {
            Ok(reply) => {
                info!(reply_len = reply.len(), "sending reply");
                if let Err(e) = gw.send_reply(&msg.sender_id, &reply, &context_token).await {
                    error!(error = %e, "failed to send reply");
                }
            }
            Err(e) => {
                error!(error = %e, "LLM query failed");
                let err_msg = format!("抱歉，处理出错了: {}", e);
                let _ = gw.send_reply(&msg.sender_id, &err_msg, &context_token).await;
            }
        }
    }

    poll_handle.await?;
    Ok(())
}
