use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("entry not found: {0}")]
    NotFound(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
