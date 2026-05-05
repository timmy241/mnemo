use std::io::{self, Write};

use tracing::info;

use crate::config::{AppConfig, ModelConfig};
use mnemo_gateway::WechatGateway;

pub async fn run() -> anyhow::Result<()> {
    let mut config = AppConfig::load().unwrap_or_default();

    // ── Step 1: Model config ───────────────────────────────────────────────
    println!("\n=== 模型配置 ===\n");

    let base_url = prompt(
        "API Base URL",
        config.model.as_ref().map(|m| m.base_url.as_str()),
        "https://api.openai.com",
    );
    let api_key = prompt(
        "API Key",
        config.model.as_ref().map(|m| m.api_key.as_str()),
        "",
    );
    let model_list_str = prompt(
        "Model list (comma-separated)",
        config
            .model
            .as_ref()
            .map(|m| m.model_list.join(",").to_string())
            .as_deref(),
        "gpt-4o-mini,gpt-4o",
    );
    let model_list: Vec<String> = model_list_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let select_model = prompt(
        "Select model",
        config.model.as_ref().map(|m| m.select_model.as_str()),
        model_list.first().map(|s| s.as_str()).unwrap_or(""),
    );

    config.model = Some(ModelConfig {
        base_url,
        api_key,
        model_list,
        select_model,
    });

    // ── Step 2: WeChat login ───────────────────────────────────────────────
    println!("\n=== 微信登录 ===\n");
    println!("接下来需要扫码登录微信，登录成功后 token 会自动保存。\n");

    let gateway = WechatGateway::new(None);
    let session = gateway.login().await?;
    config.wechat_token = Some(session);

    // ── Save ───────────────────────────────────────────────────────────────
    config.save()?;
    println!("\n✅ 配置已保存到 ~/.mnemo/config.json\n");
    info!("setup complete");

    Ok(())
}

fn prompt(label: &str, current: Option<&str>, default: &str) -> String {
    let hint = current.unwrap_or(default);
    if hint.is_empty() {
        print!("{}: ", label);
    } else {
        print!("{} [{}]: ", label, hint);
    }
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();

    if input.is_empty() {
        hint.to_string()
    } else {
        input.to_string()
    }
}
