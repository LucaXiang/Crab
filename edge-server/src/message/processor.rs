use crate::core::ServerState;
use crate::message::{BusMessage, EventType};
use crate::utils::AppError;
use async_trait::async_trait;
use shared::order::{OrderCommand, OrderCommandPayload};
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

    /// Apply price rules to items if the command is AddItems
    ///
    /// This ensures price rules are applied regardless of how the command arrives
    /// (local Tauri or remote MessageBus).
    async fn apply_price_rules_if_needed(&self, mut command: OrderCommand) -> OrderCommand {
        // Only process AddItems commands
        if let OrderCommandPayload::AddItems { order_id, items } = &command.payload {
            // Get order snapshot to find zone_id
            let snapshot = match self.state.orders_manager().get_snapshot(order_id) {
                Ok(Some(s)) => s,
                Ok(None) | Err(_) => {
                    // If order not found or error, return command unmodified
                    return command;
                }
            };

            // Determine if this is a retail order (no zone)
            let is_retail = snapshot.zone_id.is_none();
            let zone_id = snapshot.zone_id.as_deref();

            // Load applicable price rules for this zone
            let rules = self
                .state
                .price_rule_engine
                .load_rules_for_zone(zone_id, is_retail)
                .await;

            if rules.is_empty() {
                return command;
            }

            // Get current timestamp for time-based rule validation
            let current_time = chrono::Utc::now().timestamp_millis();

            // Apply price rules to items
            let processed_items = self
                .state
                .price_rule_engine
                .apply_rules(items.clone(), &rules, current_time)
                .await;

            // Update command with processed items
            command.payload = OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: processed_items,
            };
        }

        command
    }

    /// Handle order commands (order.open_table, order.add_items, etc.)
    async fn handle_order_command(
        &self,
        _action: &str,
        params: &Option<serde_json::Value>,
    ) -> Result<ProcessResult, AppError> {
        // Parse the full OrderCommand from params (preserves command_id, operator info)
        let Some(params_value) = params else {
            return Ok(ProcessResult::Failed {
                reason: "Missing params for order command".to_string(),
            });
        };

        // Parse full command (sent by client with command_id, operator_id, etc.)
        let mut command: OrderCommand = serde_json::from_value(params_value.clone())
            .map_err(|e| AppError::invalid(format!("Invalid OrderCommand: {}", e)))?;

        // Apply price rules for AddItems commands
        command = self.apply_price_rules_if_needed(command).await;

        // Execute via OrdersManager
        let response = self.state.orders_manager().execute_command(command);

        // Return result
        if response.success {
            Ok(ProcessResult::Success {
                message: "Order command executed".to_string(),
                payload: serde_json::to_value(&response).ok(),
            })
        } else {
            Ok(ProcessResult::Failed {
                reason: response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Unknown error".to_string()),
            })
        }
    }

    /// Handle sync.orders request (for reconnection)
    async fn handle_sync_orders(
        &self,
        params: &Option<serde_json::Value>,
    ) -> Result<ProcessResult, AppError> {
        // Parse since_sequence from params
        let since_sequence = params
            .as_ref()
            .and_then(|p| p.get("since_sequence"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Get events since the given sequence
        match self.state.orders_manager().get_events_since(since_sequence) {
            Ok(events) => {
                // Also get current active orders
                let active_orders = self
                    .state
                    .orders_manager()
                    .get_active_orders()
                    .unwrap_or_default();

                let current_sequence = self
                    .state
                    .orders_manager()
                    .get_current_sequence()
                    .unwrap_or(0);

                let response = serde_json::json!({
                    "events": events,
                    "active_orders": active_orders,
                    "server_sequence": current_sequence,
                    "requires_full_sync": since_sequence == 0
                });

                Ok(ProcessResult::Success {
                    message: "Sync completed".to_string(),
                    payload: Some(response),
                })
            }
            Err(e) => Ok(ProcessResult::Failed {
                reason: format!("Sync failed: {}", e),
            }),
        }
    }

    /// Handle sync.order_snapshot request - get a single order's snapshot
    async fn handle_sync_order_snapshot(
        &self,
        params: &Option<serde_json::Value>,
    ) -> Result<ProcessResult, AppError> {
        let order_id = params
            .as_ref()
            .and_then(|p| p.get("order_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::invalid("Missing order_id parameter"))?;

        match self.state.orders_manager().get_snapshot(order_id) {
            Ok(Some(snapshot)) => Ok(ProcessResult::Success {
                message: "Order snapshot retrieved".to_string(),
                payload: serde_json::to_value(&snapshot).ok(),
            }),
            Ok(None) => Ok(ProcessResult::Failed {
                reason: format!("Order not found: {}", order_id),
            }),
            Err(e) => Ok(ProcessResult::Failed {
                reason: format!("Failed to get snapshot: {}", e),
            }),
        }
    }

    /// Handle sync.active_events request - get events for active orders since sequence
    async fn handle_sync_active_events(
        &self,
        params: &Option<serde_json::Value>,
    ) -> Result<ProcessResult, AppError> {
        let since_sequence = params
            .as_ref()
            .and_then(|p| p.get("since_sequence"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        match self
            .state
            .orders_manager()
            .get_active_events_since(since_sequence)
        {
            Ok(events) => {
                let current_sequence = self
                    .state
                    .orders_manager()
                    .get_current_sequence()
                    .unwrap_or(0);

                let response = serde_json::json!({
                    "events": events,
                    "server_sequence": current_sequence
                });

                Ok(ProcessResult::Success {
                    message: "Active events retrieved".to_string(),
                    payload: Some(response),
                })
            }
            Err(e) => Ok(ProcessResult::Failed {
                reason: format!("Failed to get active events: {}", e),
            }),
        }
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
            // ========== Order Commands ==========
            action if action.starts_with("order.") => {
                self.handle_order_command(action, &payload.params).await
            }
            // ========== Sync Commands ==========
            "sync.orders" => self.handle_sync_orders(&payload.params).await,
            "sync.order_snapshot" => self.handle_sync_order_snapshot(&payload.params).await,
            "sync.active_events" => self.handle_sync_active_events(&payload.params).await,
            _ => {
                tracing::warn!("Unknown request action: {}", payload.action);
                Ok(ProcessResult::Failed {
                    reason: format!("Unknown action: {}", payload.action),
                })
            }
        }
    }
}
