use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct LlmClient {
    http: Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

impl LlmClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        let base = base_url.trim_end_matches('/');
        // If base_url already ends with /v1, don't double it
        let base = if base.ends_with("/v1") {
            base.to_string()
        } else {
            format!("{}/v1", base)
        };
        Self {
            http: Client::new(),
            base_url: base,
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn query(&self, user_message: &str) -> anyhow::Result<String> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: user_message.to_string(),
            }],
            max_tokens: 4096,
        };

        let body_json = serde_json::to_string(&body)?;
        info!(url = %url, model = %self.model, body = %body_json, "calling LLM");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .await?;

        let status = resp.status();
        let resp_text = resp.text().await?;

        if !status.is_success() {
            warn!(status = %status, body = %resp_text, "LLM error response");
            anyhow::bail!("LLM HTTP {}: {}", status, resp_text);
        }

        debug!(body_len = resp_text.len(), "LLM response body");

        let chat_resp: ChatResponse = serde_json::from_str(&resp_text).map_err(|e| {
            warn!(error = %e, body = %resp_text, "failed to parse LLM response");
            e
        })?;

        let reply = chat_resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_else(|| "(empty response)".into());

        info!(reply_len = reply.len(), "LLM reply received");
        Ok(reply)
    }
}
