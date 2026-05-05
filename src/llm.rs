use std::time::Instant;

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Default)]
pub struct LlmMetrics {
    pub ftl_ms: u64,
    pub elapsed_ms: u64,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub metrics: LlmMetrics,
}

#[derive(Debug, Clone)]
pub struct LlmClient {
    http: Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[derive(Serialize)]
struct StreamChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    stream: bool,
    stream_options: StreamOptions,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// ── SSE streaming types ────────────────────────────────────────────────

#[derive(Deserialize)]
struct StreamChunk {
    choices: Option<Vec<StreamChoice>>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

// ── Client ─────────────────────────────────────────────────────────────

impl LlmClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        let base = base_url.trim_end_matches('/');
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
        let resp = self
            .query_with_context(vec![ChatMessage {
                role: "user".into(),
                content: user_message.to_string(),
            }])
            .await?;
        Ok(resp.content)
    }

    pub async fn query_with_context(&self, messages: Vec<ChatMessage>) -> anyhow::Result<LlmResponse> {
        let url = format!("{}/chat/completions", self.base_url);

        let body = StreamChatRequest {
            model: self.model.clone(),
            messages,
            max_tokens: 4096,
            stream: true,
            stream_options: StreamOptions {
                include_usage: true,
            },
        };

        let body_json = serde_json::to_string(&body)?;
        info!(url = %url, model = %self.model, "calling LLM (streaming)");

        let start = Instant::now();

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            warn!(status = %status, body = %err_text, "LLM error response");
            anyhow::bail!("LLM HTTP {}: {}", status, err_text);
        }

        let mut stream = resp.bytes_stream();
        let mut full_reply = String::new();
        let mut first_token_latency_ms: u64 = 0;
        let mut usage = Usage::default();
        let mut got_first_token = false;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data: ") {
                    continue;
                }
                let data = &line[6..];
                if data == "[DONE]" {
                    continue;
                }

                match serde_json::from_str::<StreamChunk>(data) {
                    Ok(chunk) => {
                        if let Some(u) = chunk.usage {
                            usage = u;
                        }
                        if let Some(choices) = chunk.choices {
                            for choice in choices {
                                if let Some(content) = choice.delta.content {
                                    if !got_first_token {
                                        first_token_latency_ms = start.elapsed().as_millis() as u64;
                                        got_first_token = true;
                                    }
                                    full_reply.push_str(&content);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!(error = %e, data = %data, "failed to parse SSE chunk");
                    }
                }
            }
        }

        let elapsed_ms = start.elapsed().as_millis() as u64;

        let metrics = LlmMetrics {
            ftl_ms: first_token_latency_ms,
            elapsed_ms,
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
        };

        info!(
            ftl_ms = metrics.ftl_ms,
            elapsed_ms = metrics.elapsed_ms,
            input_tokens = metrics.input_tokens,
            output_tokens = metrics.output_tokens,
            reply_len = full_reply.len(),
            "LLM stream complete"
        );

        Ok(LlmResponse {
            content: full_reply,
            metrics,
        })
    }
}
