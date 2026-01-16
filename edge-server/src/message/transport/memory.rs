//! Memory 传输层实现 (同进程通信)

use std::sync::Arc;

use shared::message::BusMessage;
use tokio::sync::Mutex;
use tokio::sync::broadcast;

use crate::utils::AppError;

/// In-process memory transport for same-process communication
///
/// Uses tokio broadcast channel internally for zero-copy messaging.
/// 用于测试或 Oneshot 模式。
#[derive(Debug, Clone)]
pub struct MemoryTransport {
    rx: Arc<Mutex<broadcast::Receiver<BusMessage>>>,
    tx: Option<Arc<broadcast::Sender<BusMessage>>>,
}

impl MemoryTransport {
    /// Create from a message bus sender (for receiving broadcasts)
    pub fn new(tx: &broadcast::Sender<BusMessage>) -> Self {
        Self {
            rx: Arc::new(Mutex::new(tx.subscribe())),
            tx: None,
        }
    }

    /// Create with client sender for simulating client messages
    pub fn with_client_sender(
        broadcast_tx: &broadcast::Sender<BusMessage>,
        client_tx: &broadcast::Sender<BusMessage>,
    ) -> Self {
        Self {
            rx: Arc::new(Mutex::new(broadcast_tx.subscribe())),
            tx: Some(Arc::new(client_tx.clone())),
        }
    }

    pub async fn read_message(&self) -> Result<BusMessage, AppError> {
        let mut rx = self.rx.lock().await;
        rx.recv()
            .await
            .map_err(|e| AppError::internal(e.to_string()))
    }

    pub async fn write_message(&self, msg: &BusMessage) -> Result<(), AppError> {
        // Send to server via client_tx (for simulating client messages)
        if let Some(tx) = &self.tx {
            tx.send(msg.clone())
                .map_err(|e| AppError::internal(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn close(&self) -> Result<(), AppError> {
        Ok(())
    }
}
