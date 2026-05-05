use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tracing::info;

use crate::config::PgConfig;

pub async fn connect(cfg: &PgConfig) -> anyhow::Result<PgPool> {
    let url = format!(
        "postgres://{}:{}@{}:{}/{}",
        cfg.user, cfg.password, cfg.host, cfg.port, cfg.database
    );
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    info!(host = %cfg.host, db = %cfg.database, "PG connected");
    Ok(pool)
}

pub async fn fetch_context(
    pool: &PgPool,
    conversation_id: &str,
    limit: i64,
) -> anyhow::Result<Vec<(String, String)>> {
    let rows = sqlx::query(
        "SELECT role, content FROM messages \
         WHERE conversation_id = $1 \
         ORDER BY created_at DESC \
         LIMIT $2",
    )
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    // Reverse so oldest first
    let mut result: Vec<(String, String)> = rows
        .into_iter()
        .map(|r| (r.get::<String, _>("role"), r.get::<String, _>("content")))
        .collect();
    result.reverse();
    Ok(result)
}

pub async fn store_message(
    pool: &PgPool,
    conversation_id: &str,
    sender_id: &str,
    sender_name: &str,
    receiver_id: &str,
    role: &str,
    content: &str,
    platform: &str,
    platform_msg_id: Option<&str>,
    direction: &str,
    raw_context: &serde_json::Value,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO messages \
         (conversation_id, sender_id, sender_name, receiver_id, \
          role, content, platform, platform_msg_id, direction, raw_context) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(conversation_id)
    .bind(sender_id)
    .bind(sender_name)
    .bind(receiver_id)
    .bind(role)
    .bind(content)
    .bind(platform)
    .bind(platform_msg_id)
    .bind(direction)
    .bind(raw_context)
    .execute(pool)
    .await?;

    Ok(())
}

pub fn build_conversation_id(msg: &mnemo_gateway::IncomingMessage) -> String {
    match msg.platform.as_str() {
        "wechat" => format!("wechat:{}", msg.sender_id),
        "qq" => {
            let ctx = &msg.context;
            if let Some(group) = ctx.get("group_openid").and_then(|v| v.as_str()) {
                format!("qq:group:{}", group)
            } else if let Some(ch) = ctx.get("channel_id").and_then(|v| v.as_str()) {
                format!("qq:guild:{}", ch)
            } else if ctx.get("msg_type").and_then(|v| v.as_str()) == Some("dm") {
                let guild = ctx.get("guild_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("qq:dm:{}", guild)
            } else {
                format!("qq:c2c:{}", msg.sender_id)
            }
        }
        _ => format!("{}:{}", msg.platform, msg.sender_id),
    }
}
