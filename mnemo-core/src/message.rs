use async_trait::async_trait;

use crate::error::CoreError;
use crate::types::{LlmRequest, LlmResponse};

#[async_trait]
pub trait MessageProcessor: Send + Sync {
    async fn process(&self, request: LlmRequest) -> Result<LlmResponse, CoreError>;
}
