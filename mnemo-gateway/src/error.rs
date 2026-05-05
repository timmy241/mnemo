use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("connection error: {0}")]
    Connection(String),

    #[error("message error: {0}")]
    Message(String),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
