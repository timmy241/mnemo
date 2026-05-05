use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::info;

use mnemo_gateway::qq::types::QQChannelConfig;
use mnemo_gateway::TokenSession;

const CONFIG_DIR: &str = ".mnemo";
const CONFIG_FILE: &str = "config.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    /// Legacy field — kept for backward compatibility.
    #[serde(default)]
    pub wechat_token: Option<TokenSession>,
    #[serde(default)]
    pub channels: Vec<ChannelConfig>,
    #[serde(default)]
    pub model: Option<ModelConfig>,
    #[serde(default)]
    pub pg: Option<PgConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChannelConfig {
    #[serde(rename = "wechat")]
    Wechat { token: TokenSession },
    #[serde(rename = "qq")]
    QQ(QQChannelConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub base_url: String,
    pub api_key: String,
    pub model_list: Vec<String>,
    pub select_model: String,
}

impl AppConfig {
    fn config_path() -> PathBuf {
        dirs_or_default().join(CONFIG_FILE)
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            info!(path = %path.display(), "config file not found, using defaults");
            return Ok(Self::default());
        }
        let data = fs::read_to_string(&path)?;
        let mut config: AppConfig = serde_json::from_str(&data)?;
        info!(path = %path.display(), "config loaded");

        // Migrate legacy wechat_token into channels
        if config.channels.is_empty() {
            if let Some(token) = config.wechat_token.take() {
                info!("migrating wechat_token to channels");
                config.channels.push(ChannelConfig::Wechat { token });
            }
        }

        Ok(config)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        info!(path = %path.display(), "config saved");
        Ok(())
    }
}

fn dirs_or_default() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CONFIG_DIR)
}
