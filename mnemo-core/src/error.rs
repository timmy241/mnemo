use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("LLM error: {0}")]
    Llm(String),

    #[error("memory error: {0}")]
    Memory(#[from] mnemo_memory::MemoryError),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
