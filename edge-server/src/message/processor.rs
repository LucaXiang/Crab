//! Message Processor Trait
//!
//! Provides a pluggable architecture for message processing with ACID guarantees.

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::common::AppError;
use crate::message::{BusMessage, EventType};

/// Result of message processing
#[derive(Debug, Clone)]
pub enum ProcessResult {
    /// Message processed successfully
    Success { message: String },
    /// Message processing failed, should retry
    Retry { reason: String, retry_count: u32 },
    /// Message processing failed permanently, do not retry
    Failed { reason: String },
    /// Message skipped (e.g., duplicate)
    Skipped { reason: String },
}

impl ProcessResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ProcessResult::Success { .. })
    }

    pub fn should_retry(&self) -> bool {
        matches!(self, ProcessResult::Retry { .. })
    }
}

/// Message Processor trait
///
/// Implement this trait to create custom message processors with:
/// - ACID transaction support
/// - Idempotency checks
/// - Retry logic
/// - Error recovery
#[async_trait]
pub trait MessageProcessor: Send + Sync {
    /// Get the event type this processor handles
    fn event_type(&self) -> EventType;

    /// Process a message with ACID guarantees
    ///
    /// # Implementation Guidelines
    ///
    /// 1. **Idempotency**: Check if message was already processed
    /// 2. **Atomicity**: Use database transactions
    /// 3. **Consistency**: Validate data before processing
    /// 4. **Isolation**: Use appropriate transaction isolation levels
    /// 5. **Durability**: Commit to database before returning Success
    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError>;

    /// Check if a message was already processed (idempotency)
    ///
    /// Default implementation returns false (always process).
    /// Override for idempotency support.
    async fn is_duplicate(&self, _msg: &BusMessage) -> Result<bool, AppError> {
        Ok(false)
    }

    /// Maximum retry attempts for this processor
    fn max_retries(&self) -> u32 {
        3
    }

    /// Delay between retries (in milliseconds)
    fn retry_delay_ms(&self) -> u64 {
        1000
    }
}

/// Order intent processor - handles client requests and updates order state
///
/// 处理客户端请求，维护服务端订单状态，然后广播 OrderSync
pub struct OrderIntentProcessor;

#[async_trait]
impl MessageProcessor for OrderIntentProcessor {
    fn event_type(&self) -> EventType {
        EventType::OrderIntent
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: Value = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid payload: {}", e)))?;

        let action = payload["action"].as_str().unwrap_or("unknown");
        let _data = &payload["data"];

        tracing::info!(
            event = "order_intent",
            action = %action,
            "Processing order intent"
        );

        // TODO: ACID 事务处理订单状态
        //
        // let mut tx = db.begin().await?;
        //
        // // 1. 幂等性检查
        // if self.is_duplicate(msg).await? {
        //     return Ok(ProcessResult::Skipped {
        //         reason: "Duplicate intent".to_string(),
        //     });
        // }
        //
        // // 2. 根据 action 处理不同的业务逻辑
        // match action {
        //     "add_dish" => {
        //         // 验证菜品
        //         let dishes = validate_dishes(&data["dishes"]).await?;
        //         // 更新订单
        //         let order = db.add_dishes_to_order(table_id, dishes, &tx).await?;
        //         // 广播状态
        //         broadcast_table_sync("dish_added", order, &tx).await?;
        //     }
        //     "payment" => {
        //         // 处理付款
        //         let payment = process_payment(&data, &tx).await?;
        //         // 广播状态
        //         broadcast_table_sync("payment_completed", payment, &tx).await?;
        //     }
        //     "checkout" => {
        //         // 结账
        //         let checkout = finalize_order(table_id, &tx).await?;
        //         // 广播状态
        //         broadcast_table_sync("order_closed", checkout, &tx).await?;
        //     }
        //     _ => return Ok(ProcessResult::Failed {
        //         reason: format!("Unknown action: {}", action),
        //     }),
        // }
        //
        // // 3. 标记消息已处理
        // db.mark_processed(msg_id, &tx).await?;
        //
        // // 4. 提交事务
        // tx.commit().await?;

        Ok(ProcessResult::Success {
            message: format!("Table intent processed: {}", action),
        })
    }

    fn max_retries(&self) -> u32 {
        5 // 订单操作重试 5 次
    }

    fn retry_delay_ms(&self) -> u64 {
        2000 // 2 秒后重试
    }
}

/// Data sync processor - handles base data updates
///
/// 处理菜品原型数据更新（价格、名称、图片、状态等）
pub struct DataSyncProcessor;

#[async_trait]
impl MessageProcessor for DataSyncProcessor {
    fn event_type(&self) -> EventType {
        EventType::DataSync
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: Value = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid payload: {}", e)))?;

        let sync_type = payload["sync_type"].as_str().unwrap_or("unknown");
        let _data = &payload["data"];

        tracing::info!(
            event = "data_sync",
            sync_type = %sync_type,
            "Processing data sync"
        );

        // TODO: ACID 事务处理数据更新
        //
        // let mut tx = db.begin().await?;
        //
        // match sync_type {
        //     "dish_price" => {
        //         // 更新菜品价格
        //         let dish_id = data["dish_id"].as_str().ok_or(AppError::invalid("Missing dish_id".into()))?;
        //         let new_price = data["new_price"].as_u64().ok_or(AppError::invalid("Missing new_price".into()))?;
        //         db.update_dish_price(dish_id, new_price, &tx).await?;
        //
        //         // 记录价格历史
        //         db.insert_price_history(dish_id, old_price, new_price, &tx).await?;
        //     }
        //     "dish_sold_out" => {
        //         // 菜品沽清
        //         let dish_id = data["dish_id"].as_str().ok_or(AppError::invalid("Missing dish_id".into()))?;
        //         db.set_dish_available(dish_id, false, &tx).await?;
        //     }
        //     "dish_added" => {
        //         // 新菜品上架
        //         db.insert_dish(data, &tx).await?;
        //     }
        //     _ => return Ok(ProcessResult::Failed {
        //         reason: format!("Unknown sync type: {}", sync_type),
        //     }),
        // }
        //
        // // 标记已处理
        // db.mark_processed(msg_id, &tx).await?;
        //
        // // 提交事务
        // tx.commit().await?;

        Ok(ProcessResult::Success {
            message: format!("Data sync processed: {}", sync_type),
        })
    }

    fn max_retries(&self) -> u32 {
        3 // 数据同步重试 3 次
    }
}

/// Notification processor - handles system notifications
pub struct NotificationProcessor;

#[async_trait]
impl MessageProcessor for NotificationProcessor {
    fn event_type(&self) -> EventType {
        EventType::Notification
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        let payload: Value = msg
            .parse_payload()
            .map_err(|e| AppError::invalid(format!("Invalid payload: {}", e)))?;

        let title = payload["title"].as_str().unwrap_or("(no title)");
        let body = payload["body"].as_str().unwrap_or("(no body)");

        tracing::info!(
            event = "notification",
            title = %title,
            body = %body,
            "Processing notification"
        );

        // TODO: 通知逻辑
        // - 记录日志
        // - 发送推送
        // - 邮件/短信通知

        Ok(ProcessResult::Success {
            message: format!("Notification processed: {}", title),
        })
    }
}

/// Server command processor - handles commands from upstream/central server
///
/// 处理上层服务器发来的指令（配置更新、数据同步指令、远程控制等）
pub struct ServerCommandProcessor {
    state: Arc<crate::server::ServerState>,
}

impl ServerCommandProcessor {
    pub fn new(state: Arc<crate::server::ServerState>) -> Self {
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
            .map_err(|e| AppError::invalid(format!("Invalid payload: {}", e)))?;

        match payload.command {
            shared::message::ServerCommand::Activate {
                tenant_id,
                tenant_name,
                edge_id,
                edge_name,
                tenant_ca_pem,
                edge_cert_pem,
                edge_key_pem,
            } => {
                tracing::info!(
                    "🚀 Received Activate command: tenant={}, edge={}",
                    tenant_name,
                    edge_name
                );

                // Save certificates to filesystem
                if let Err(e) = self
                    .state
                    .save_certificates(&tenant_ca_pem, &edge_cert_pem, &edge_key_pem)
                    .await
                {
                    tracing::error!("Failed to save certificates: {}", e);
                    return Ok(ProcessResult::Failed {
                        reason: format!("Failed to save certificates: {}", e),
                    });
                }

                // Calculate certificate fingerprint using SHA256
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(edge_cert_pem.as_bytes());
                let cert_fingerprint = hex::encode(hasher.finalize());

                // Update activation state in database
                match self
                    .state
                    .activate_with_metadata(
                        &tenant_id,
                        &tenant_name,
                        &edge_id,
                        &edge_name,
                        &cert_fingerprint,
                    )
                    .await
                {
                    Ok(_) => Ok(ProcessResult::Success {
                        message: format!(
                            "Server activated: tenant={}, edge={}",
                            tenant_name, edge_name
                        ),
                    }),
                    Err(e) => {
                        tracing::error!("Activation failed: {}", e);
                        Ok(ProcessResult::Failed {
                            reason: format!("Activation failed: {}", e),
                        })
                    }
                }
            }
            shared::message::ServerCommand::Ping => {
                Ok(ProcessResult::Success {
                    message: "Pong".to_string(),
                })
            }
            shared::message::ServerCommand::ConfigUpdate { key, value } => {
                tracing::info!("Received ConfigUpdate: {} = {:?}", key, value);
                // TODO: Implement config update logic
                Ok(ProcessResult::Skipped {
                    reason: "Config update not implemented yet".to_string(),
                })
            }
            shared::message::ServerCommand::SyncData { data_type, force } => {
                tracing::info!("Received SyncData: {:?}, force={}", data_type, force);
                // TODO: Implement data sync logic
                Ok(ProcessResult::Skipped {
                    reason: "Data sync not implemented yet".to_string(),
                })
            }
            shared::message::ServerCommand::Restart {
                delay_seconds,
                reason,
            } => {
                tracing::warn!(
                    "Received Restart command: delay={}s, reason={:?}",
                    delay_seconds,
                    reason
                );
                // TODO: Implement graceful restart
                Ok(ProcessResult::Skipped {
                    reason: "Restart not implemented yet".to_string(),
                })
            }
        }
    }
}
