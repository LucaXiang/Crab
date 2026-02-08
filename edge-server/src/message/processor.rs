use crate::core::ServerState;
use crate::db::repository::{employee, role, system_issue};
use crate::message::{BusMessage, EventType};
use crate::orders::actions::open_table::load_matching_rules;
use shared::error::AppError;
use async_trait::async_trait;
use shared::order::{OrderCommand, OrderCommandPayload};
use std::sync::Arc;

/// 获取执行订单命令所需的权限
fn get_required_permission(payload: &OrderCommandPayload) -> Option<&'static str> {
    match payload {
        // 敏感操作需要权限
        OrderCommandPayload::VoidOrder { .. } => Some("orders:void"),
        OrderCommandPayload::CompItem { .. } | OrderCommandPayload::UncompItem { .. } => {
            Some("orders:comp")
        }
        OrderCommandPayload::ApplyOrderDiscount { .. }
        | OrderCommandPayload::ApplyOrderSurcharge { .. } => Some("orders:discount"),
        OrderCommandPayload::CancelPayment { .. } => Some("orders:refund"),
        OrderCommandPayload::RemoveItem { .. } => Some("orders:cancel_item"),
        OrderCommandPayload::MoveOrder { .. } => Some("tables:transfer"),
        OrderCommandPayload::MergeOrders { .. } => Some("tables:merge_bill"),
        // 基础操作无需特殊权限
        _ => None,
    }
}

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
                match system_issue::create(
                    &self.state.pool,
                    shared::models::SystemIssueCreate {
                        source: "remote".to_string(),
                        kind: kind.clone(),
                        blocking: *blocking,
                        target: target.clone(),
                        params: params.clone(),
                        title: title.clone(),
                        description: description.clone(),
                        options: options.clone(),
                    },
                )
                .await
                {
                    Ok(issue) => {
                        let id_str = issue.id.to_string();
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

    /// 检查操作者是否拥有指定权限
    async fn check_operator_permission(&self, operator_id: i64, permission: &str) -> bool {
        // 查询员工信息
        let employee = match employee::find_by_id(&self.state.pool, operator_id).await {
            Ok(Some(emp)) => emp,
            Ok(None) => {
                tracing::warn!(operator_id = %operator_id, "Operator not found");
                return false;
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to query operator");
                return false;
            }
        };

        // 查询角色权限
        let role = match role::find_by_id(&self.state.pool, employee.role_id).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                tracing::warn!(role_id = %employee.role_id, "Role not found");
                return false;
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to query role");
                return false;
            }
        };

        // 检查权限
        // 1. admin 角色或拥有 "all" 权限的用户拥有所有权限
        if role.name == "admin" || role.permissions.iter().any(|p| p == "all") {
            return true;
        }

        // 2. 精确匹配
        if role.permissions.iter().any(|p| p == permission) {
            return true;
        }

        // 3. 通配符匹配 (e.g., "orders:*" matches "orders:void")
        if let Some(prefix) = permission.split(':').next() {
            let wildcard = format!("{}:*", prefix);
            if role.permissions.iter().any(|p| p == &wildcard) {
                return true;
            }
        }

        false
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

        // 权限检查：敏感命令需要验证操作者权限
        if let Some(required_permission) = get_required_permission(&command.payload) {
            let has_permission =
                self.check_operator_permission(command.operator_id, required_permission)
                    .await;
            if !has_permission {
                tracing::warn!(
                    operator_id = %command.operator_id,
                    operator_name = %command.operator_name,
                    required_permission = required_permission,
                    command = ?std::mem::discriminant(&command.payload),
                    "Permission denied: operator lacks required permission"
                );
                return Ok(ProcessResult::Failed {
                    reason: format!("Permission denied: requires {} permission", required_permission),
                });
            }
        }

        // 保存需要加载规则的命令信息
        let rule_load_info = match &command.payload {
            OrderCommandPayload::OpenTable {
                zone_id, is_retail, ..
            } => Some((*zone_id, *is_retail)),
            _ => None,
        };
        // MoveOrder: 保存移桌信息用于规则重新加载
        let move_order_info = if let OrderCommandPayload::MoveOrder {
            order_id,
            target_zone_id,
            ..
        } = &command.payload
        {
            Some((order_id.clone(), *target_zone_id))
        } else {
            None
        };

        // Execute via OrdersManager (CatalogService is injected, metadata lookup is automatic)
        let response = self.state.orders_manager().execute_command(command);

        if response.success {
            // OpenTable 成功后加载并缓存价格规则
            if let Some((zone_id, is_retail)) = rule_load_info
                && let Some(ref order_id) = response.order_id
            {
                let rules =
                    load_matching_rules(&self.state.pool, zone_id, is_retail).await;
                if !rules.is_empty() {
                    tracing::debug!(
                        order_id = %order_id,
                        rule_count = rules.len(),
                        "Cached order price rules"
                    );
                    self.state.orders_manager().cache_rules(order_id, rules);
                }
            }

            // MoveOrder 成功后：用新区域重新加载规则
            if let Some((ref order_id, ref target_zone_id)) = move_order_info {
                // 从 snapshot 获取 is_retail（移桌不改变 is_retail）
                if let Ok(Some(snapshot)) = self.state.orders_manager().get_snapshot(order_id) {
                    let rules = load_matching_rules(
                        &self.state.pool,
                        *target_zone_id,
                        snapshot.is_retail,
                    )
                    .await;
                    tracing::debug!(
                        order_id = %order_id,
                        target_zone_id = ?target_zone_id,
                        rule_count = rules.len(),
                        "Reloaded zone rules after table move"
                    );
                    self.state.orders_manager().cache_rules(order_id, rules);
                }
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
