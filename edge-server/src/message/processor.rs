use crate::common::AppError;
use crate::message::{BusMessage, EventType};
use crate::server::ServerState;
use async_trait::async_trait;
use std::sync::Arc;

/// 消息处理结果
#[derive(Debug)]
pub enum ProcessResult {
    /// 处理成功
    Success {
        message: String,
        payload: Option<serde_json::Value>,
    },
    /// 处理失败
    Failed { reason: String },
    /// 跳过处理
    Skipped { reason: String },
    /// 需要重试
    Retry {
        reason: String,
        retry_count: Option<u32>,
    },
}

/// 消息处理器特征
///
/// 实现此特征以处理特定类型的消息总线事件。
#[async_trait]
pub trait MessageProcessor: Send + Sync {
    /// 获取此处理器处理的事件类型
    fn event_type(&self) -> EventType;

    /// 处理消息
    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError>;

    /// 最大重试次数 (默认: 3)
    fn max_retries(&self) -> u32 {
        3
    }

    /// 重试延迟 (毫秒, 默认: 1000)
    fn retry_delay_ms(&self) -> u64 {
        1000
    }
}

/// 通知消息处理器
///
/// 处理 Notification 事件，通常只是记录日志。
pub struct NotificationProcessor;

#[async_trait]
impl MessageProcessor for NotificationProcessor {
    fn event_type(&self) -> EventType {
        EventType::Notification
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: shared::message::NotificationPayload = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid notification payload: {}", e)))?;

        tracing::info!(
            "🔔 Notification [{}]: {} - {}",
            payload.level,
            payload.title,
            payload.message
        );

        Ok(ProcessResult::Success {
            message: format!("Notification '{}' logged", payload.title),
            payload: None,
        })
    }
}

/// 服务器指令处理器
///
/// 处理来自上层服务器的指令 (ServerCommand)。
pub struct ServerCommandProcessor {
    state: Arc<ServerState>,
}

impl ServerCommandProcessor {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl MessageProcessor for ServerCommandProcessor {
    fn event_type(&self) -> EventType {
        EventType::ServerCommand
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: shared::message::ServerCommandPayload = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid server command payload: {}", e)))?;

        tracing::info!("⚙️ Received server command: {:?}", payload.command);

        match &payload.command {
            shared::message::ServerCommand::Ping => {
                tracing::info!("Server Ping received");
            }
            shared::message::ServerCommand::Restart {
                delay_seconds,
                reason,
            } => {
                tracing::info!(
                    "Server restart requested in {}s. Reason: {:?}",
                    delay_seconds,
                    reason
                );
                // Trigger restart logic (via state or event)
                // For now, just log it. In real implementation, we'd use self.state to signal shutdown.
                // self.state.shutdown_token().cancel(); // Example
                let _ = self.state; // Suppress unused warning for now until implemented
            }
            _ => {
                tracing::warn!("Unimplemented server command: {:?}", payload.command);
            }
        }

        Ok(ProcessResult::Success {
            message: "Server command processed".to_string(),
            payload: None,
        })
    }
}

/// 客户端请求处理器 - 处理来自客户端的 RPC 请求
pub struct RequestCommandProcessor {
    state: Arc<ServerState>,
}

impl RequestCommandProcessor {
    pub fn new(state: Arc<ServerState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl MessageProcessor for RequestCommandProcessor {
    fn event_type(&self) -> EventType {
        EventType::RequestCommand
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: shared::message::RequestCommandPayload = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid payload: {}", e)))?;

        tracing::info!(
            request_id = %msg.request_id,
            action = %payload.action,
            "Processing RPC request"
        );

        // 处理具体的请求动作
        match payload.action.as_str() {
            "ping" => {
                tracing::info!("Client ping received");
                Ok(ProcessResult::Success {
                    message: "Pong".to_string(),
                    payload: None,
                })
            }
            "echo" => Ok(ProcessResult::Success {
                message: "Echo".to_string(),
                payload: payload.params,
            }),
            "status" => {
                let status = serde_json::json!({
                    "activated": self.state.is_activated().await,
                    "version": env!("CARGO_PKG_VERSION"),
                    "server_time": chrono::Utc::now().to_rfc3339()
                });

                Ok(ProcessResult::Success {
                    message: "Server Status".to_string(),
                    payload: Some(status),
                })
            }
            _ => {
                tracing::warn!("Unknown request action: {}", payload.action);
                Ok(ProcessResult::Failed {
                    reason: format!("Unknown action: {}", payload.action),
                })
            }
        }
    }
}
