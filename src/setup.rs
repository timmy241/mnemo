use std::io::{self, Write};

use tracing::info;

use crate::config::{AppConfig, ChannelConfig, ModelConfig, PgConfig};
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

    // ── Step 2: Channel selection ──────────────────────────────────────────
    let has_wechat = config.channels.iter().any(|c| matches!(c, ChannelConfig::Wechat { .. }));
    let has_qq = config.channels.iter().any(|c| matches!(c, ChannelConfig::QQ(_)));

    let wechat_mark = if has_wechat { " ●" } else { "" };
    let qq_mark = if has_qq { " ●" } else { "" };

    println!("\n=== 消息通道 ===\n");
    println!("请选择要配置的消息通道：");
    println!("  1. 微信 (WeChat){}", wechat_mark);
    println!("  2. QQ{}", qq_mark);
    println!("  3. 跳过");
    println!();

    let choice = prompt("选择通道", None, "1");
    match choice.as_str() {
        "1" => setup_wechat(&mut config).await?,
        "2" => setup_qq(&mut config).await?,
        "3" => println!("跳过通道配置。\n"),
        _ => println!("未知选项，跳过通道配置。\n"),
    }

    // ── Step 3: PostgreSQL config ──────────────────────────────────────────
    println!("\n=== PostgreSQL 配置 ===\n");
    println!("用于存储对话记忆，直接回车跳过（不启用记忆功能）。\n");

    let pg_host = prompt(
        "Host",
        config.pg.as_ref().map(|p| p.host.as_str()),
        "localhost",
    );
    let pg_port_str = prompt(
        "Port",
        config.pg.as_ref().map(|p| p.port.to_string()).as_deref(),
        "5432",
    );
    let pg_user = prompt(
        "User",
        config.pg.as_ref().map(|p| p.user.as_str()),
        "postgres",
    );
    let pg_password = prompt(
        "Password",
        config.pg.as_ref().map(|p| p.password.as_str()),
        "",
    );
    let pg_database = prompt(
        "Database",
        config.pg.as_ref().map(|p| p.database.as_str()),
        "mnemo",
    );

    if !pg_password.is_empty() {
        let port: u16 = pg_port_str.parse().unwrap_or(5432);
        config.pg = Some(PgConfig {
            host: pg_host,
            port,
            user: pg_user,
            password: pg_password,
            database: pg_database,
        });
    } else {
        println!("跳过 PostgreSQL 配置。\n");
    }

    // ── Save ───────────────────────────────────────────────────────────────
    config.save()?;
    println!("\n✅ 配置已保存到 ~/.mnemo/config.json\n");
    info!("setup complete");

    Ok(())
}

async fn setup_wechat(config: &mut AppConfig) -> anyhow::Result<()> {
    println!("\n=== 微信登录 ===\n");
    println!("接下来需要扫码登录微信，登录成功后 token 会自动保存。\n");

    let gateway = WechatGateway::new(None);
    let session = gateway.login().await?;

    // Remove existing wechat channel if any
    config.channels.retain(|c| !matches!(c, ChannelConfig::Wechat { .. }));
    config.channels.push(ChannelConfig::Wechat { token: session });

    Ok(())
}

async fn setup_qq(config: &mut AppConfig) -> anyhow::Result<()> {
    println!("\n=== QQ Bot 配置 ===\n");
    println!("请在 QQ 开放平台 (https://q.qq.com) 创建机器人，获取 AppID 和 Secret。\n");

    let app_id = prompt("AppID", None, "");
    let secret = prompt("Secret", None, "");

    // Remove existing qq channel if any
    config.channels.retain(|c| !matches!(c, ChannelConfig::QQ(_)));
    config.channels.push(ChannelConfig::QQ(
        mnemo_gateway::qq::types::QQChannelConfig { app_id, secret },
    ));

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
