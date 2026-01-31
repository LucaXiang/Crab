use crate::core::ServerState;
use crate::db::repository::system_issue::{CreateSystemIssue, SystemIssueRepository};
use crate::message::{BusMessage, EventType};
use crate::orders::actions::open_table::load_matching_rules;
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

        match &payload.command {
            shared::message::ServerCommand::Ping => {}
            shared::message::ServerCommand::Restart {
                delay_seconds,
                reason,
            } => {
                tracing::warn!("Server restart requested in {}s. Reason: {:?}", delay_seconds, reason);
            }
            shared::message::ServerCommand::SystemIssue {
                kind,
                blocking,
                target,
                params,
                title,
                description,
                options,
            } => {
                tracing::info!("Remote system issue received: kind={}", kind);
                let repo = SystemIssueRepository::new(self.state.get_db());
                match repo
                    .create(CreateSystemIssue {
                        source: "remote".to_string(),
                        kind: kind.clone(),
                        blocking: *blocking,
                        target: target.clone(),
                        params: params.clone(),
                        title: title.clone(),
                        description: description.clone(),
                        options: options.clone(),
                    })
                    .await
                {
                    Ok(issue) => {
                        // 广播 sync 事件，前端实时感知新 system_issue
                        let id_str = issue
                            .id
                            .as_ref()
                            .map(|r| r.to_string())
                            .unwrap_or_default();
                        self.state
                            .broadcast_sync("system_issue", "created", &id_str, Some(&issue))
                            .await;
                    }
                    Err(e) => {
                        tracing::error!("Failed to create remote system_issue: {:?}", e);
                    }
                }
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
        let command: OrderCommand = serde_json::from_value(params_value.clone())
            .map_err(|e| AppError::invalid(format!("Invalid OrderCommand: {}", e)))?;

        // 保存 OpenTable 的信息用于后续规则加载
        let open_table_info = if let OrderCommandPayload::OpenTable {
            zone_id, is_retail, ..
        } = &command.payload
        {
            Some((zone_id.clone(), *is_retail))
        } else {
            None
        };

        // 检查是否是 RestoreOrder 命令
        let restore_order_id =
            if let OrderCommandPayload::RestoreOrder { order_id } = &command.payload {
                Some(order_id.clone())
            } else {
                None
            };

        // Execute via OrdersManager (CatalogService is injected, metadata lookup is automatic)
        let response = self.state.orders_manager().execute_command(command);

        // 如果是 OpenTable 且成功执行，加载并缓存价格规则
        if response.success {
            if let Some((zone_id, is_retail)) = open_table_info
                && let Some(ref order_id) = response.order_id
            {
                // 加载匹配的价格规则
                let rules =
                    load_matching_rules(&self.state.get_db(), zone_id.as_deref(), is_retail, self.state.config.timezone).await;

                // 缓存到 OrdersManager
                if !rules.is_empty() {
                    tracing::debug!(
                        order_id = %order_id,
                        rule_count = rules.len(),
                        "缓存订单价格规则"
                    );
                    self.state.orders_manager().cache_rules(order_id, rules);
                }
            }

            // 如果是 RestoreOrder 且成功执行，重新加载并缓存价格规则
            if let Some(order_id) = restore_order_id {
                tracing::debug!(
                    order_id = %order_id,
                    "订单恢复成功，重新加载价格规则"
                );
                self.state.load_rules_for_order(&order_id).await;
            }
        }

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


        // 处理具体的请求动作
        match payload.action.as_str() {
            "ping" => {
                tracing::trace!("Client ping received");
                // 返回 epoch 以便客户端检测服务器重启
                let pong_payload = serde_json::json!({
                    "epoch": &self.state.epoch,
                    "server_time": shared::util::now_millis()
                });
                Ok(ProcessResult::Success {
                    message: "Pong".to_string(),
                    payload: Some(pong_payload),
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
                    "server_time": shared::util::now_millis()
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
