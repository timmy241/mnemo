use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::GatewayError;
use crate::types::{IncomingMessage, OutgoingMessage};

#[async_trait]
pub trait Gateway: Send + Sync {
    async fn start(&self, tx: mpsc::Sender<IncomingMessage>) -> Result<(), GatewayError>;
    async fn send(&self, message: OutgoingMessage) -> Result<(), GatewayError>;
    async fn stop(&self) -> Result<(), GatewayError>;
}
