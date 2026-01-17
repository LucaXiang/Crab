// crab-client/src/message/mod.rs
// 消息模块

pub use shared::message::{BusMessage, EventType};

/// Error type for message client operations
#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Request timed out: {0}")]
    Timeout(String),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
}
