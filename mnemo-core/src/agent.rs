use std::sync::Arc;

use tracing::info;

use crate::error::CoreError;
use crate::message::MessageProcessor;
use crate::types::{AgentConfig, AgentEvent, LlmMessage, LlmRequest};
use mnemo_memory::{MemoryEntry, MemoryQuery, MemoryStore};
use chrono::Utc;
use uuid::Uuid;

pub struct AgentLoop {
    config: AgentConfig,
    memory: Arc<dyn MemoryStore>,
    processor: Arc<dyn MessageProcessor>,
}

impl AgentLoop {
    pub fn new(
        config: AgentConfig,
        memory: Arc<dyn MemoryStore>,
        processor: Arc<dyn MessageProcessor>,
    ) -> Self {
        Self {
            config,
            memory,
            processor,
        }
    }

    pub async fn handle_message(
        &self,
        session_id: &str,
        user_message: &str,
    ) -> Result<(String, AgentEvent), CoreError> {
        // Store user message in memory
        let user_entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "user".into(),
            content: user_message.to_string(),
            created_at: Utc::now(),
            metadata: serde_json::Value::Null,
        };
        self.memory.store(user_entry).await?;

        // Retrieve recent context
        let context = self
            .memory
            .query(MemoryQuery {
                session_id: session_id.to_string(),
                limit: Some(50),
                before: None,
            })
            .await?;

        // Build messages for LLM
        let mut messages = vec![LlmMessage {
            role: "system".into(),
            content: self.config.system_prompt.clone(),
        }];
        for entry in &context {
            messages.push(LlmMessage {
                role: entry.role.clone(),
                content: entry.content.clone(),
            });
        }

        // Call LLM
        let request = LlmRequest {
            messages,
            config: self.config.clone(),
        };

        let response = self.processor.process(request).await?;

        // Store assistant response
        let assistant_entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "assistant".into(),
            content: response.content.clone(),
            created_at: Utc::now(),
            metadata: serde_json::to_value(&response.usage)?,
        };
        self.memory.store(assistant_entry).await?;

        info!(
            session_id = session_id,
            input_tokens = response.usage.input_tokens,
            output_tokens = response.usage.output_tokens,
            "message processed"
        );

        Ok((
            response.content.clone(),
            AgentEvent::Completed {
                session_id: session_id.to_string(),
            },
        ))
    }
}
