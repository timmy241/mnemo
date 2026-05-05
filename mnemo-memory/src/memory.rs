use async_trait::async_trait;

use crate::error::MemoryError;
use crate::types::{MemoryEntry, MemoryQuery};

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn store(&self, entry: MemoryEntry) -> Result<(), MemoryError>;
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError>;
    async fn delete(&self, id: &str) -> Result<(), MemoryError>;
    async fn clear_session(&self, session_id: &str) -> Result<(), MemoryError>;
}
