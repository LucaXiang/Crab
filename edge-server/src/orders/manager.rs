//! OrdersManager - Core command processing and event generation
//!
//! This module handles:
//! - Command validation and processing
//! - Event generation with global sequence numbers
//! - Persistence to redb (transactional)
//! - Snapshot updates
//! - Event broadcasting (via callback)
//!
//! # Command Flow
//!
//! ```text
//! execute_command(cmd)
//!     ├─ 1. Idempotency check (command_id)
//!     ├─ 2. Begin write transaction
//!     ├─ 3. Create CommandContext
//!     ├─ 4. Convert command to action and execute
//!     ├─ 5. Apply events to snapshots via EventApplier
//!     ├─ 6. Persist events and snapshots
//!     ├─ 7. Mark command processed
//!     ├─ 8. Commit transaction
//!     ├─ 9. Broadcast event(s)
//!     └─ 10. Return response
//! ```

use super::actions::CommandAction;
use super::appliers::EventAction;
use super::money;
use super::storage::{OrderStorage, StorageError};
use super::traits::{CommandContext, CommandHandler, CommandMetadata, EventApplier, OrderError};
use crate::db::models::PriceRule;
use crate::services::catalog_service::ProductMeta;
use chrono::Utc;
use chrono_tz::Tz;
use shared::order::{
    CommandError, CommandErrorCode, CommandResponse, OrderCommand, OrderEvent, OrderSnapshot,
    OrderStatus,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use thiserror::Error;
use tokio::sync::broadcast;

/// Manager errors
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Order already completed: {0}")]
    OrderAlreadyCompleted(String),

    #[error("Order already voided: {0}")]
    OrderAlreadyVoided(String),

    #[error("Item not found: {0}")]
    ItemNotFound(String),

    #[error("Payment not found: {0}")]
    PaymentNotFound(String),

    #[error("Insufficient quantity")]
    InsufficientQuantity,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Table is already occupied: {0}")]
    TableOccupied(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// 将存储错误转换为错误码（前端负责本地化）
fn classify_storage_error(e: &StorageError) -> CommandErrorCode {
    // 先按枚举变体精确匹配
    match e {
        StorageError::Serialization(_) => return CommandErrorCode::InternalError,
        StorageError::OrderNotFound(_) => return CommandErrorCode::OrderNotFound,
        StorageError::EventNotFound(_, _) => return CommandErrorCode::InternalError,
        _ => {}
    }

    // redb 错误通过字符串匹配分类
    let err_str = e.to_string().to_lowercase();

    // 磁盘空间不足
    if err_str.contains("no space") || err_str.contains("disk full") || err_str.contains("enospc")
    {
        return CommandErrorCode::StorageFull;
    }

    // 内存不足
    if err_str.contains("out of memory") || err_str.contains("cannot allocate") {
        return CommandErrorCode::OutOfMemory;
    }

    // 数据损坏
    if err_str.contains("corrupt") || err_str.contains("invalid database") {
        return CommandErrorCode::StorageCorrupted;
    }

    // 默认：系统繁忙（redb 的 Database/Transaction/Table/Storage/Commit 错误）
    CommandErrorCode::SystemBusy
}

impl From<ManagerError> for CommandError {
    fn from(err: ManagerError) -> Self {
        let (code, message) = match err {
            ManagerError::Storage(e) => {
                let code = classify_storage_error(&e);
                let message = e.to_string(); // 保留技术细节用于日志/调试
                tracing::error!(error = %e, error_code = ?code, "Storage error occurred");
                (code, message)
            }
            ManagerError::OrderNotFound(id) => (
                CommandErrorCode::OrderNotFound,
                format!("Order not found: {}", id),
            ),
            ManagerError::OrderAlreadyCompleted(id) => (
                CommandErrorCode::OrderAlreadyCompleted,
                format!("Order already completed: {}", id),
            ),
            ManagerError::OrderAlreadyVoided(id) => (
                CommandErrorCode::OrderAlreadyVoided,
                format!("Order already voided: {}", id),
            ),
            ManagerError::ItemNotFound(id) => (
                CommandErrorCode::ItemNotFound,
                format!("Item not found: {}", id),
            ),
            ManagerError::PaymentNotFound(id) => (
                CommandErrorCode::PaymentNotFound,
                format!("Payment not found: {}", id),
            ),
            ManagerError::InsufficientQuantity => (
                CommandErrorCode::InsufficientQuantity,
                "Insufficient quantity".to_string(),
            ),
            ManagerError::InvalidAmount => (
                CommandErrorCode::InvalidAmount,
                "Invalid amount".to_string(),
            ),
            ManagerError::InvalidOperation(msg) => (CommandErrorCode::InvalidOperation, msg),
            ManagerError::TableOccupied(msg) => (CommandErrorCode::TableOccupied, msg),
            ManagerError::Internal(msg) => (CommandErrorCode::InternalError, msg),
        };
        CommandError::new(code, message)
    }
}

impl From<OrderError> for ManagerError {
    fn from(err: OrderError) -> Self {
        match err {
            OrderError::OrderNotFound(id) => ManagerError::OrderNotFound(id),
            OrderError::OrderAlreadyCompleted(id) => ManagerError::OrderAlreadyCompleted(id),
            OrderError::OrderAlreadyVoided(id) => ManagerError::OrderAlreadyVoided(id),
            OrderError::ItemNotFound(id) => ManagerError::ItemNotFound(id),
            OrderError::PaymentNotFound(id) => ManagerError::PaymentNotFound(id),
            OrderError::InsufficientQuantity => ManagerError::InsufficientQuantity,
            OrderError::InvalidAmount => ManagerError::InvalidAmount,
            OrderError::InvalidOperation(msg) => ManagerError::InvalidOperation(msg),
            OrderError::TableOccupied(msg) => ManagerError::TableOccupied(msg),
            OrderError::Storage(msg) => ManagerError::Internal(msg),
        }
    }
}

pub type ManagerResult<T> = Result<T, ManagerError>;

/// Event broadcast channel capacity (支持高并发: 10000订单 × 4事件)
const EVENT_CHANNEL_CAPACITY: usize = 65536;

/// OrdersManager for command processing
///
/// The `epoch` field is a unique identifier generated on each startup.
/// Clients use it to detect server restarts and trigger full resync.
pub struct OrdersManager {
    storage: OrderStorage,
    event_tx: broadcast::Sender<OrderEvent>,
    /// Server instance epoch - unique ID generated on startup
    /// Used by clients to detect server restarts
    epoch: String,
    /// Cached rules per order
    rule_cache: Arc<RwLock<HashMap<String, Vec<PriceRule>>>>,
    /// Catalog service for product metadata lookup
    catalog_service: Option<Arc<crate::services::CatalogService>>,
    /// Archive service for completed orders (optional, only set when SurrealDB is available)
    archive_service: Option<super::OrderArchiveService>,
    /// 业务时区
    tz: Tz,
}

impl std::fmt::Debug for OrdersManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrdersManager")
            .field("storage", &"<OrderStorage>")
            .field("event_tx", &"<broadcast::Sender>")
            .field("epoch", &self.epoch)
            .finish()
    }
}

impl OrdersManager {
    /// Create a new OrdersManager with the given database path
    pub fn new(db_path: impl AsRef<Path>, tz: Tz) -> ManagerResult<Self> {
        let storage = OrderStorage::open(db_path)?;
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let epoch = uuid::Uuid::new_v4().to_string();
        tracing::info!(epoch = %epoch, "OrdersManager started with new epoch");
        Ok(Self {
            storage,
            event_tx,
            epoch,
            rule_cache: Arc::new(RwLock::new(HashMap::new())),
            catalog_service: None,
            archive_service: None,
            tz,
        })
    }

    /// Set the catalog service for product metadata lookup
    pub fn set_catalog_service(&mut self, catalog_service: Arc<crate::services::CatalogService>) {
        self.catalog_service = Some(catalog_service);
    }

    /// Set the archive service for SurrealDB integration
    pub fn set_archive_service(&mut self, db: surrealdb::Surreal<surrealdb::engine::local::Db>) {
        self.archive_service = Some(super::OrderArchiveService::new(db, self.tz));
    }

    /// Generate next receipt number (crash-safe via redb)
    fn next_receipt_number(&self) -> String {
        let count = self.storage.next_order_count().unwrap_or(1);
        let date_str = Utc::now().with_timezone(&self.tz).format("%Y%m%d").to_string();
        format!("FAC{}{}", date_str, 10000 + count)
    }

    /// Create an OrdersManager with existing storage (for testing)
    #[cfg(test)]
    pub fn with_storage(storage: OrderStorage) -> Self {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let epoch = uuid::Uuid::new_v4().to_string();
        Self {
            storage,
            event_tx,
            epoch,
            rule_cache: Arc::new(RwLock::new(HashMap::new())),
            catalog_service: None,
            archive_service: None,
            tz: chrono_tz::Europe::Madrid,
        }
    }

    /// Get the server epoch (unique instance ID)
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// 缓存并持久化订单的价格规则快照
    ///
    /// 开台时调用，将规则同时写入内存缓存和 redb，
    /// 确保重启后能从 redb 恢复而非重新查询数据库。
    pub fn cache_rules(&self, order_id: &str, rules: Vec<PriceRule>) {
        // 持久化到 redb
        if let Err(e) = self.storage.store_rule_snapshot(order_id, &rules) {
            tracing::error!(order_id = %order_id, error = %e, "持久化规则快照失败，该订单的规则定格保证降级");
        }
        // 写入内存缓存
        let mut cache = self.rule_cache.write();
        cache.insert(order_id.to_string(), rules);
    }

    /// Get cached rules for an order
    pub fn get_cached_rules(&self, order_id: &str) -> Option<Vec<PriceRule>> {
        let cache = self.rule_cache.read();
        cache.get(order_id).cloned()
    }

    /// 清除订单的规则缓存和 redb 快照
    ///
    /// 订单终结时 (Complete/Void/Move/Merge) 调用。
    pub fn remove_cached_rules(&self, order_id: &str) {
        // 清除内存缓存
        {
            let mut cache = self.rule_cache.write();
            cache.remove(order_id);
        }
        // 清除 redb 快照
        if let Err(e) = self.storage.remove_rule_snapshot(order_id) {
            tracing::error!(order_id = %order_id, error = %e, "清除规则快照失败");
        }
    }

    /// 从 redb 恢复所有规则快照到内存缓存 (启动预热用)
    ///
    /// 自动清理孤儿快照（订单已终结但规则快照未清除的情况）。
    /// 返回恢复的订单数量。
    pub fn restore_rule_snapshots_from_redb(&self) -> usize {
        let snapshots = match self.storage.get_all_rule_snapshots() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "从 redb 恢复规则快照失败");
                return 0;
            }
        };

        // 获取活跃订单 ID 集合，用于清理孤儿快照
        let active_ids: std::collections::HashSet<String> = self
            .storage
            .get_active_order_ids()
            .unwrap_or_default()
            .into_iter()
            .collect();

        let mut restored = 0;
        let mut orphaned = 0;
        let mut cache = self.rule_cache.write();

        for (order_id, rules) in snapshots {
            if active_ids.contains(&order_id) {
                cache.insert(order_id, rules);
                restored += 1;
            } else {
                // 孤儿快照：订单已终结但规则未清除（可能是崩溃导致）
                if let Err(e) = self.storage.remove_rule_snapshot(&order_id) {
                    tracing::warn!(order_id = %order_id, error = %e, "清理孤儿规则快照失败");
                }
                orphaned += 1;
            }
        }

        if orphaned > 0 {
            tracing::info!(orphaned, "清理了孤儿规则快照");
        }

        restored
    }

    /// Subscribe to event broadcasts
    pub fn subscribe(&self) -> broadcast::Receiver<OrderEvent> {
        self.event_tx.subscribe()
    }

    /// Get the underlying storage
    pub fn storage(&self) -> &OrderStorage {
        &self.storage
    }

    /// Get the archive service if configured
    pub fn archive_service(&self) -> Option<&super::OrderArchiveService> {
        self.archive_service.as_ref()
    }

    /// Execute a command and return the response
    pub fn execute_command(&self, cmd: OrderCommand) -> CommandResponse {
        match self.process_command(cmd.clone()) {
            Ok((response, events)) => {
                // Broadcast events after successful commit
                for event in events {
                    let _ = self.event_tx.send(event);
                }
                response
            }
            Err(err) => CommandResponse::error(cmd.command_id, err.into()),
        }
    }

    /// Execute a command and return both the response and generated events
    ///
    /// This is useful for Tauri integration where events need to be emitted to the frontend.
    /// Unlike `execute_command`, this returns the events to the caller while still
    /// broadcasting them internally.
    pub fn execute_command_with_events(
        &self,
        cmd: OrderCommand,
    ) -> (CommandResponse, Vec<OrderEvent>) {
        match self.process_command(cmd.clone()) {
            Ok((response, events)) => {
                // Broadcast events after successful commit
                for event in &events {
                    let _ = self.event_tx.send(event.clone());
                }
                (response, events)
            }
            Err(err) => (CommandResponse::error(cmd.command_id, err.into()), vec![]),
        }
    }

    /// Get product metadata for items from CatalogService
    fn get_product_metadata_for_items(
        &self,
        items: &[shared::order::CartItemInput],
    ) -> HashMap<String, ProductMeta> {
        let Some(catalog) = &self.catalog_service else {
            return HashMap::new();
        };
        let product_ids: Vec<String> = items.iter().map(|i| i.product_id.clone()).collect();
        catalog.get_product_meta_batch(&product_ids)
    }

    /// Process command and return response with events
    ///
    /// Uses the action-based architecture:
    /// 1. Convert command to CommandAction
    /// 2. Execute action to generate events
    /// 3. Apply events to snapshots via EventApplier
    /// 4. Persist everything atomically
    fn process_command(
        &self,
        cmd: OrderCommand,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        tracing::info!(command_id = %cmd.command_id, payload = ?cmd.payload, "Processing command");
        
        // 1. Idempotency check (before transaction)
        if self.storage.is_command_processed(&cmd.command_id)? {
            tracing::warn!(command_id = %cmd.command_id, "Duplicate command");
            return Ok((CommandResponse::duplicate(cmd.command_id), vec![]));
        }

        // 2. For OpenTable: pre-check table availability before generating receipt_number
        // This avoids wasting receipt numbers on failed table opens
        if let shared::order::OrderCommandPayload::OpenTable { table_id: Some(tid), table_name, .. } = &cmd.payload
            && let Some(existing) = self.storage.find_active_order_for_table(tid)? {
                let name = table_name.as_deref().unwrap_or(tid);
                return Err(ManagerError::TableOccupied(format!(
                    "桌台 {} 已被占用 (订单: {})", name, existing
                )));
            }

        // 3. Pre-generate receipt_number and queue_number for OpenTable (BEFORE transaction to avoid deadlock)
        // redb doesn't allow nested write transactions
        let pre_generated_receipt = match &cmd.payload {
            shared::order::OrderCommandPayload::OpenTable { .. } => {
                let receipt = self.next_receipt_number();
                tracing::info!(receipt_number = %receipt, "Pre-generated receipt number");
                Some(receipt)
            }
            _ => None,
        };
        let pre_generated_queue = match &cmd.payload {
            shared::order::OrderCommandPayload::OpenTable { is_retail: true, .. } => {
                match self.storage.next_queue_number(self.tz) {
                    Ok(qn) => {
                        tracing::info!(queue_number = qn, "Pre-generated queue number");
                        Some(qn)
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to generate queue number");
                        None
                    }
                }
            }
            _ => None,
        };

        // 3. Begin write transaction
        let txn = self.storage.begin_write()?;

        // Double-check idempotency within transaction
        if self
            .storage
            .is_command_processed_txn(&txn, &cmd.command_id)?
        {
            return Ok((CommandResponse::duplicate(cmd.command_id), vec![]));
        }

        // 4. Get current sequence for context initialization
        let current_sequence = self.storage.get_current_sequence()?;

        // 5. Create context and metadata
        let mut ctx = CommandContext::new(&txn, &self.storage, current_sequence);
        let metadata = CommandMetadata {
            command_id: cmd.command_id.clone(),
            operator_id: cmd.operator_id.clone(),
            operator_name: cmd.operator_name.clone(),
            timestamp: cmd.timestamp,
        };

        // 6. Convert to action and execute
        // For OpenTable: use pre-generated receipt_number
        // For AddItems: inject cached price rules and product metadata from CatalogService
        let action: CommandAction = match &cmd.payload {
            shared::order::OrderCommandPayload::OpenTable {
                table_id,
                table_name,
                zone_id,
                zone_name,
                guest_count,
                is_retail,
            } => {
                tracing::info!(table_id = ?table_id, table_name = ?table_name, "Processing OpenTable command");
                // Use pre-generated receipt_number (generated before transaction)
                let receipt_number = pre_generated_receipt.ok_or_else(|| {
                    OrderError::InvalidOperation("receipt_number must be pre-generated for OpenTable".to_string())
                })?;
                CommandAction::OpenTable(super::actions::OpenTableAction {
                    table_id: table_id.clone(),
                    table_name: table_name.clone(),
                    zone_id: zone_id.clone(),
                    zone_name: zone_name.clone(),
                    guest_count: *guest_count,
                    is_retail: *is_retail,
                    queue_number: pre_generated_queue,
                    receipt_number,
                })
            }
            shared::order::OrderCommandPayload::AddItems { order_id, items } => {
                let rules = self.get_cached_rules(order_id).unwrap_or_default();
                let product_metadata = self.get_product_metadata_for_items(items);
                CommandAction::AddItems(super::actions::AddItemsAction {
                    order_id: order_id.clone(),
                    items: items.clone(),
                    rules,
                    product_metadata,
                })
            }
            _ => (&cmd).into(),
        };
        let events = futures::executor::block_on(action.execute(&mut ctx, &metadata))
            .map_err(ManagerError::from)?;

        // 6. Apply events to snapshots and update active order tracking
        for event in &events {
            // Load or create snapshot for this order
            let mut snapshot = ctx
                .load_snapshot(&event.order_id)
                .unwrap_or_else(|_| OrderSnapshot::new(event.order_id.clone()));

            // Apply event using EventApplier
            let applier: EventAction = event.into();
            applier.apply(&mut snapshot, event);

            // Save updated snapshot to context
            ctx.save_snapshot(snapshot);
        }

        // 7. Persist events
        for event in &events {
            self.storage.store_event(&txn, event)?;
        }

        // 8. Persist snapshots and update active order tracking
        for snapshot in ctx.modified_snapshots() {
            self.storage.store_snapshot(&txn, snapshot)?;

            // Update active order tracking based on status
            match snapshot.status {
                OrderStatus::Active => {
                    self.storage.mark_order_active(&txn, &snapshot.order_id)?;
                }
                OrderStatus::Completed
                | OrderStatus::Void
                | OrderStatus::Merged
                | OrderStatus::Moved => {
                    self.storage.mark_order_inactive(&txn, &snapshot.order_id)?;
                    // Queue for archive if archive service is configured
                    if self.archive_service.is_some() {
                        self.storage.queue_for_archive(&txn, &snapshot.order_id)?;
                    }
                }
            }
        }

        // 9. Update sequence counter
        let max_sequence = events
            .iter()
            .map(|e| e.sequence)
            .max()
            .unwrap_or(current_sequence);
        if max_sequence > current_sequence {
            self.storage.set_sequence(&txn, max_sequence)?;
        }

        // 10. Mark command processed
        self.storage.mark_command_processed(&txn, &cmd.command_id)?;

        // 11. Commit transaction
        txn.commit().map_err(StorageError::from)?;

        // 12. Clean up rule cache for terminal orders (Complete/Void/Move/Merge)
        match &cmd.payload {
            shared::order::OrderCommandPayload::CompleteOrder { order_id, .. }
            | shared::order::OrderCommandPayload::VoidOrder { order_id, .. }
            | shared::order::OrderCommandPayload::MoveOrder { order_id, .. } => {
                self.remove_cached_rules(order_id);
            }
            shared::order::OrderCommandPayload::MergeOrders { source_order_id, .. } => {
                self.remove_cached_rules(source_order_id);
            }
            _ => {}
        }

        // 13. Return response
        // Note: Archive is now handled by ArchiveWorker listening to event broadcasts
        let order_id = events.first().map(|e| e.order_id.clone());
        tracing::info!(command_id = %cmd.command_id, order_id = ?order_id, event_count = events.len(), "Command processed successfully");
        Ok((CommandResponse::success(cmd.command_id, order_id), events))
    }

    // ========== Public Query Methods ==========

    /// Get a snapshot by order ID
    pub fn get_snapshot(&self, order_id: &str) -> ManagerResult<Option<OrderSnapshot>> {
        let mut snapshot = self.storage.get_snapshot(order_id)?;
        // 确保 line_total 已计算
        if let Some(ref mut order) = snapshot {
            let needs_recalc = order.items.iter().any(|item| item.line_total.is_none());
            if needs_recalc {
                money::recalculate_totals(order);
            }
        }
        Ok(snapshot)
    }

    /// Get all active order snapshots
    ///
    /// Ensures all items have `line_total` computed for consistency with order totals.
    pub fn get_active_orders(&self) -> ManagerResult<Vec<OrderSnapshot>> {
        let mut orders = self.storage.get_active_orders()?;
        // 确保 line_total 已计算
        for order in &mut orders {
            let needs_recalc = order.items.iter().any(|item| item.line_total.is_none());
            if needs_recalc {
                money::recalculate_totals(order);
            }
        }
        Ok(orders)
    }

    /// Get current sequence number
    pub fn get_current_sequence(&self) -> ManagerResult<u64> {
        Ok(self.storage.get_current_sequence()?)
    }

    /// Get events since a given sequence
    pub fn get_events_since(&self, since_sequence: u64) -> ManagerResult<Vec<OrderEvent>> {
        Ok(self.storage.get_events_since(since_sequence)?)
    }

    /// Get events for active orders since a given sequence
    pub fn get_active_events_since(&self, since_sequence: u64) -> ManagerResult<Vec<OrderEvent>> {
        Ok(self.storage.get_active_events_since(since_sequence)?)
    }

    /// Rebuild a snapshot from events (for verification)
    ///
    /// Uses EventApplier to apply each event to build the snapshot.
    pub fn rebuild_snapshot(&self, order_id: &str) -> ManagerResult<OrderSnapshot> {
        let events = self.storage.get_events_for_order(order_id)?;
        if events.is_empty() {
            return Err(ManagerError::OrderNotFound(order_id.to_string()));
        }

        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        for event in &events {
            let applier: EventAction = event.into();
            applier.apply(&mut snapshot, event);
        }

        Ok(snapshot)
    }
}

// Make OrdersManager Clone-able via Arc
impl Clone for OrdersManager {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            event_tx: self.event_tx.clone(),
            epoch: self.epoch.clone(),
            rule_cache: self.rule_cache.clone(),
            catalog_service: self.catalog_service.clone(),
            archive_service: self.archive_service.clone(),
            tz: self.tz,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::types::ServiceType;
    use shared::order::{CartItemInput, OrderCommandPayload, OrderEventType, PaymentInput, VoidType};

    fn create_test_manager() -> OrdersManager {
        let storage = OrderStorage::open_in_memory().unwrap();
        OrdersManager::with_storage(storage)
    }

    fn create_open_table_cmd(operator_id: &str) -> OrderCommand {
        OrderCommand::new(
            operator_id.to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
            },
        )
    }

    #[test]
    fn test_open_table() {
        let manager = create_test_manager();
        let cmd = create_open_table_cmd("op-1");

        let response = manager.execute_command(cmd);

        assert!(response.success);
        assert!(response.order_id.is_some());

        let order_id = response.order_id.unwrap();
        let snapshot = manager.get_snapshot(&order_id).unwrap();
        assert!(snapshot.is_some());

        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.status, OrderStatus::Active);
        assert_eq!(snapshot.table_id, Some("T1".to_string()));
    }

    #[test]
    fn test_idempotency() {
        let manager = create_test_manager();
        let cmd = create_open_table_cmd("op-1");

        let response1 = manager.execute_command(cmd.clone());
        assert!(response1.success);
        let _order_id = response1.order_id.clone();

        // Execute same command again
        let response2 = manager.execute_command(cmd);
        assert!(response2.success);
        assert_eq!(response2.order_id, None); // Duplicate returns no order_id

        // Should still only have one order
        let orders = manager.get_active_orders().unwrap();
        assert_eq!(orders.len(), 1);
    }

    #[test]
    fn test_add_items() {
        let manager = create_test_manager();

        // Open table
        let open_cmd = create_open_table_cmd("op-1");
        let open_response = manager.execute_command(open_cmd);
        let order_id = open_response.order_id.unwrap();

        // Add items
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Test Product".to_string(),
                    price: 10.0,
                    original_price: None,
                    quantity: 2,
                    selected_options: None,
                    selected_specification: None,
                    manual_discount_percent: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );

        let response = manager.execute_command(add_cmd);
        assert!(response.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].quantity, 2);
        assert_eq!(snapshot.subtotal, 20.0);
    }

    #[test]
    fn test_add_payment_and_complete() {
        let manager = create_test_manager();

        // Open table
        let open_cmd = create_open_table_cmd("op-1");
        let open_response = manager.execute_command(open_cmd);
        let order_id = open_response.order_id.unwrap();

        // Add items
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Test Product".to_string(),
                    price: 10.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: None,
                    selected_specification: None,
                    manual_discount_percent: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        // Add payment
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: Some(20.0),
                    note: None,
                },
            },
        );
        let pay_response = manager.execute_command(pay_cmd);
        assert!(pay_response.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 10.0);
        assert_eq!(snapshot.payments.len(), 1);
        assert_eq!(snapshot.payments[0].change, Some(10.0));

        // Complete order (receipt_number comes from snapshot)
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let complete_response = manager.execute_command(complete_cmd);
        assert!(complete_response.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert!(!snapshot.receipt_number.is_empty()); // Server-generated at OpenTable
    }

    #[test]
    fn test_void_order() {
        let manager = create_test_manager();

        // Open table
        let open_cmd = create_open_table_cmd("op-1");
        let open_response = manager.execute_command(open_cmd);
        let order_id = open_response.order_id.unwrap();

        // Void order
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: Some("Customer cancelled".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let void_response = manager.execute_command(void_cmd);
        assert!(void_response.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Void);

        // Order should no longer be active
        let active_orders = manager.get_active_orders().unwrap();
        assert!(active_orders.is_empty());
    }

    #[test]
    fn test_event_broadcast() {
        let manager = create_test_manager();
        let mut rx = manager.subscribe();

        // Open table
        let open_cmd = create_open_table_cmd("op-1");
        let _ = manager.execute_command(open_cmd);

        // Should receive event
        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type, OrderEventType::TableOpened);
    }

    // ========================================================================
    // Helper: open a table with items
    // ========================================================================

    fn open_table_with_items(
        manager: &OrdersManager,
        table_id: &str,
        items: Vec<CartItemInput>,
    ) -> String {
        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(table_id.to_string()),
                table_name: Some(format!("Table {}", table_id)),
                zone_id: Some("zone:z1".to_string()),
                zone_name: Some("Zone A".to_string()),
                guest_count: 2,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        assert!(resp.success, "Failed to open table");
        let order_id = resp.order_id.unwrap();

        if !items.is_empty() {
            let add_cmd = OrderCommand::new(
                "op-1".to_string(),
                "Test Operator".to_string(),
                OrderCommandPayload::AddItems {
                    order_id: order_id.clone(),
                    items,
                },
            );
            let resp = manager.execute_command(add_cmd);
            assert!(resp.success, "Failed to add items");
        }

        order_id
    }

    fn simple_item(product_id: &str, name: &str, price: f64, quantity: i32) -> CartItemInput {
        CartItemInput {
            product_id: product_id.to_string(),
            name: name.to_string(),
            price,
            original_price: None,
            quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    // ========================================================================
    // 1. rebuild_snapshot 一致性验证
    // ========================================================================

    #[test]
    fn test_rebuild_snapshot_matches_stored() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-rebuild",
            vec![
                simple_item("product:p1", "Coffee", 4.5, 2),
                simple_item("product:p2", "Tea", 3.0, 1),
            ],
        );

        // Add a payment
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 5.0,
                    tendered: Some(10.0),
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        // Get stored snapshot
        let stored = manager.get_snapshot(&order_id).unwrap().unwrap();

        // Rebuild from events
        let rebuilt = manager.rebuild_snapshot(&order_id).unwrap();

        // Core fields should match
        assert_eq!(stored.order_id, rebuilt.order_id);
        assert_eq!(stored.status, rebuilt.status);
        assert_eq!(stored.items.len(), rebuilt.items.len());
        assert_eq!(stored.payments.len(), rebuilt.payments.len());
        assert_eq!(stored.paid_amount, rebuilt.paid_amount);
        assert_eq!(stored.table_id, rebuilt.table_id);
        assert_eq!(stored.last_sequence, rebuilt.last_sequence);
        assert_eq!(stored.state_checksum, rebuilt.state_checksum);
    }

    // ========================================================================
    // 2. MoveOrder — zone 信息正确更新
    // ========================================================================

    #[test]
    fn test_move_order_zone_updates_correctly() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-move-1",
            vec![simple_item("product:p1", "Coffee", 5.0, 1)],
        );

        // Verify initial zone
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.zone_id, Some("zone:z1".to_string()));
        assert_eq!(snapshot.zone_name, Some("Zone A".to_string()));

        // Move to a different table in a different zone
        let move_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: "T-move-2".to_string(),
                target_table_name: "Table T-move-2".to_string(),
                target_zone_id: Some("zone:z2".to_string()),
                target_zone_name: Some("Zone B".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.table_id, Some("T-move-2".to_string()));
        assert_eq!(
            snapshot.zone_id,
            Some("zone:z2".to_string()),
            "zone_id should be updated after MoveOrder"
        );
        assert_eq!(
            snapshot.zone_name,
            Some("Zone B".to_string()),
            "zone_name should be updated after MoveOrder"
        );
    }

    // ========================================================================
    // 3. Merge 带支付的订单 — 存在支付记录时拒绝合并
    // ========================================================================

    #[test]
    fn test_merge_orders_source_with_payment_rejected() {
        let manager = create_test_manager();

        // Source order with items and partial payment
        let source_id = open_table_with_items(
            &manager,
            "T-merge-src",
            vec![simple_item("product:p1", "Coffee", 10.0, 2)],
        );

        // Pay partially on source
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: source_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 5.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let source_before = manager.get_snapshot(&source_id).unwrap().unwrap();
        assert_eq!(source_before.paid_amount, 5.0);

        // Target order
        let target_id = open_table_with_items(
            &manager,
            "T-merge-tgt",
            vec![simple_item("product:p2", "Tea", 8.0, 1)],
        );

        // Merge source → target should be rejected
        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: source_id.clone(),
                target_order_id: target_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(!resp.success, "存在支付记录的订单不能合并");

        // Source and target should remain unchanged
        let source_after = manager.get_snapshot(&source_id).unwrap().unwrap();
        assert_eq!(source_after.paid_amount, 5.0);
        assert_eq!(source_after.status, OrderStatus::Active);

        let target_after = manager.get_snapshot(&target_id).unwrap().unwrap();
        assert_eq!(target_after.items.len(), 1, "Target should be unchanged");
        assert_eq!(target_after.paid_amount, 0.0);
    }

    // ========================================================================
    // 4. AddPayment 超额支付
    // ========================================================================

    #[test]
    fn test_add_payment_overpay_is_allowed() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-overpay",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // Pay way more than the total
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10000.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        // Documenting: AddPayment currently allows unlimited overpayment
        assert!(
            resp.success,
            "AddPayment allows overpayment (no upper bound check)"
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 10000.0);
    }

    // ========================================================================
    // 5. cancel_payment → re-pay → complete 完整流程
    // ========================================================================

    #[test]
    fn test_cancel_payment_then_repay_then_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-cancel-repay",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // Pay with CARD
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        // Get payment_id
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let payment_id = snapshot.payments[0].payment_id.clone();
        assert_eq!(snapshot.paid_amount, 10.0);

        // Cancel the payment
        let cancel_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CancelPayment {
                order_id: order_id.clone(),
                payment_id,
                reason: Some("Wrong card".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(cancel_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 0.0, "After cancel, paid should be 0");
        assert!(snapshot.payments[0].cancelled);

        // Re-pay with CASH
        let repay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: Some(20.0),
                    note: None,
                },
            },
        );
        manager.execute_command(repay_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 10.0);
        assert_eq!(snapshot.payments.len(), 2);

        // Complete
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
    }

    // ========================================================================
    // 6. 空订单 complete
    // ========================================================================

    #[test]
    fn test_complete_empty_order() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(&manager, "T-empty", vec![]);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        // Zero-total orders should complete (e.g., complimentary)
        assert!(resp.success, "Zero-total order should complete successfully");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
    }

    // ========================================================================
    // 7. Sequence 单调递增
    // ========================================================================

    #[test]
    fn test_sequence_monotonically_increasing() {
        let manager = create_test_manager();
        let mut rx = manager.subscribe();

        let order_id = open_table_with_items(
            &manager,
            "T-seq",
            vec![simple_item("product:p1", "Coffee", 5.0, 1)],
        );

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p2", "Tea", 3.0, 1)],
            },
        );
        manager.execute_command(add_cmd);

        let mut sequences = Vec::new();
        while let Ok(event) = rx.try_recv() {
            sequences.push(event.sequence);
        }

        assert!(sequences.len() >= 3, "Should have at least 3 events");
        for window in sequences.windows(2) {
            assert!(
                window[1] > window[0],
                "Sequences must be strictly increasing: {} should be > {}",
                window[1],
                window[0]
            );
        }
    }

    // ========================================================================
    // 8. 重复打开相同桌台应失败
    // ========================================================================

    #[test]
    fn test_open_same_table_twice_fails() {
        let manager = create_test_manager();

        let cmd1 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-dup".to_string()),
                table_name: Some("Table Dup".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
            },
        );
        let resp1 = manager.execute_command(cmd1);
        assert!(resp1.success);

        let cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-dup".to_string()),
                table_name: Some("Table Dup".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 3,
                is_retail: false,
            },
        );
        let resp2 = manager.execute_command(cmd2);
        assert!(!resp2.success, "Opening the same table twice should fail");
    }

    // ========================================================================
    // 9. void 已 void 的订单应失败
    // ========================================================================

    #[test]
    fn test_void_already_voided_order_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(&manager, "T-void-twice", vec![]);

        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(void_cmd);
        assert!(resp.success);

        let void_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp2 = manager.execute_command(void_cmd2);
        assert!(!resp2.success, "Voiding an already voided order should fail");
    }

    // ========================================================================
    // 10. 移桌后结账完整流程
    // ========================================================================

    #[test]
    fn test_move_order_then_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-mc-1",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let move_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: "T-mc-2".to_string(),
                target_table_name: "Table 2".to_string(),
                target_zone_id: None,
                target_zone_name: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(resp.success);

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert_eq!(snapshot.table_id, Some("T-mc-2".to_string()));
    }

    // ========================================================================
    // 11. split by items → complete 流程
    // ========================================================================

    #[test]
    fn test_split_by_items_then_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-si",
            vec![
                simple_item("product:p1", "Coffee", 10.0, 2),
                simple_item("product:p2", "Tea", 8.0, 1),
            ],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 28.0);
        let coffee_instance = snapshot.items[0].instance_id.clone();

        // Split pay: 2x Coffee = 20.0
        let split_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByItems {
                order_id: order_id.clone(),
                payment_method: "CASH".to_string(),
                items: vec![shared::order::SplitItem {
                    instance_id: coffee_instance,
                    name: "Coffee".to_string(),
                    quantity: 2,
                    unit_price: 10.0,
                }],
                tendered: None,
            },
        );
        let resp = manager.execute_command(split_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 20.0);

        // Pay remaining: Tea = 8.0
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: 8.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
    }

    // ========================================================================
    // 12. AA split 完整流程 → complete
    // ========================================================================

    #[test]
    fn test_aa_split_full_flow_then_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-aa",
            vec![simple_item("product:p1", "Coffee", 30.0, 1)],
        );

        // Start AA: 3 shares, pay 1
        let start_aa_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::StartAaSplit {
                order_id: order_id.clone(),
                total_shares: 3,
                shares: 1,
                payment_method: "CASH".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(start_aa_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.aa_total_shares, Some(3));
        assert_eq!(snapshot.aa_paid_shares, 1);
        assert_eq!(snapshot.paid_amount, 10.0);

        // Pay share 2
        let pay_aa_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::PayAaSplit {
                order_id: order_id.clone(),
                shares: 1,
                payment_method: "CARD".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(pay_aa_cmd);
        assert!(resp.success);

        // Pay share 3 (last — should get exact remaining)
        let pay_aa_last = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::PayAaSplit {
                order_id: order_id.clone(),
                shares: 1,
                payment_method: "CASH".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(pay_aa_last);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.aa_paid_shares, 3);
        assert_eq!(snapshot.paid_amount, 30.0);

        // Complete
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
    }

    // ========================================================================
    // 13. 零售订单应生成 queue_number
    // ========================================================================

    #[test]
    fn test_retail_order_gets_queue_number() {
        let manager = create_test_manager();

        let cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: None,
                table_name: None,
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: true,
            },
        );
        let resp = manager.execute_command(cmd);
        assert!(resp.success);

        let order_id = resp.order_id.unwrap();
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.queue_number.is_some(), "Retail order should have queue number");
        assert!(snapshot.is_retail);
    }

    // ========================================================================
    // 14. execute_command_with_events 返回 events
    // ========================================================================

    #[test]
    fn test_execute_command_with_events_returns_events() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let (resp, events) = manager.execute_command_with_events(cmd);

        assert!(resp.success);
        assert!(!events.is_empty());
        assert_eq!(events[0].event_type, OrderEventType::TableOpened);
    }

    // ========================================================================
    // 15. get_events_since 完整性
    // ========================================================================

    #[test]
    fn test_get_events_since_completeness() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-events",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let seq_before = manager.get_current_sequence().unwrap();

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let events = manager.get_events_since(seq_before).unwrap();
        assert!(!events.is_empty());
        assert!(events.iter().any(|e| e.event_type == OrderEventType::PaymentAdded));
    }

    // ========================================================================
    // 16. 合并后源订单变为 Merged 状态
    // ========================================================================

    #[test]
    fn test_merge_source_becomes_merged_status() {
        let manager = create_test_manager();

        let source_id = open_table_with_items(
            &manager,
            "T-ms",
            vec![simple_item("product:p1", "Coffee", 5.0, 1)],
        );
        let target_id = open_table_with_items(
            &manager,
            "T-mt",
            vec![simple_item("product:p2", "Tea", 3.0, 1)],
        );

        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: source_id.clone(),
                target_order_id: target_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(resp.success);

        let source = manager.get_snapshot(&source_id).unwrap().unwrap();
        assert_eq!(source.status, OrderStatus::Merged);

        let active = manager.get_active_orders().unwrap();
        assert!(active.iter().all(|o| o.order_id != source_id));

        let target = manager.get_snapshot(&target_id).unwrap().unwrap();
        assert_eq!(target.status, OrderStatus::Active);
        assert_eq!(target.items.len(), 2);
    }

    // ========================================================================
    // 17. 操作不存在的订单应返回错误
    // ========================================================================

    #[test]
    fn test_operations_on_nonexistent_order_fail() {
        let manager = create_test_manager();

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: "nonexistent".to_string(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: "nonexistent".to_string(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(!resp.success);

        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: "nonexistent".to_string(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(void_cmd);
        assert!(!resp.success);
    }

    // ========================================================================
    // 18. 完成订单后不能添加商品
    // ========================================================================

    #[test]
    fn test_cannot_add_items_to_completed_order() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-closed",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd);

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p2", "Tea", 5.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "Should not allow adding items to completed order");
    }

    // ========================================================================
    // ========================================================================
    //  边界测试: 价格/数量/折扣/支付的极端值
    // ========================================================================
    // ========================================================================

    // ========================================================================
    // 19. 零价格商品可以正常添加和完成
    // ========================================================================

    #[test]
    fn test_add_items_with_zero_price() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-zero-price",
            vec![simple_item("product:p1", "Free Sample", 0.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].price, 0.0);
        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);

        // 零总额可以直接完成
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success, "Zero-price order should complete");
    }

    // ========================================================================
    // 20. NaN 价格 — 静默变成 0 (当前行为记录)
    // ========================================================================

    #[test]
    fn test_add_items_with_nan_price_rejected() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-nan-price".to_string()),
                table_name: Some("Table NaN".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "NaN Item", f64::NAN, 2)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "NaN price should be rejected by validation");
    }

    // ========================================================================
    // 21. Infinity 价格 — 静默变成 0
    // ========================================================================

    #[test]
    fn test_add_items_with_infinity_price_rejected() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-inf-price".to_string()),
                table_name: Some("Table Inf".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Infinity Item", f64::INFINITY, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "Infinity price should be rejected by validation");
    }

    // ========================================================================
    // 22. 负价格 — 当前被 clamp 到 0
    // ========================================================================

    #[test]
    fn test_add_items_with_negative_price_rejected() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-neg-price".to_string()),
                table_name: Some("Table Neg".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Negative Item", -10.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "Negative price should be rejected by validation");
    }

    // ========================================================================
    // 23. 极大价格 × 数量仍正确计算
    // ========================================================================

    #[test]
    fn test_add_items_large_price_and_quantity() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-large",
            vec![simple_item("product:p1", "Expensive Item", 99999.99, 100)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 99999.99 * 100 = 9_999_999.0
        assert_eq!(snapshot.subtotal, 9_999_999.0);
        assert_eq!(snapshot.total, 9_999_999.0);
    }

    // ========================================================================
    // 24. f64::MAX 价格 — 转为 0 (Decimal 转换失败)
    // ========================================================================

    #[test]
    fn test_add_items_with_f64_max_price_rejected() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-f64max".to_string()),
                table_name: Some("Table Max".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Max Item", f64::MAX, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "f64::MAX price should be rejected (exceeds max)");
    }

    // ========================================================================
    // 25. 数量为 0 — 当前被接受（应添加商品但金额为 0）
    // ========================================================================

    #[test]
    fn test_add_items_with_zero_quantity_rejected() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-zero-qty".to_string()),
                table_name: Some("Table Zero Qty".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Zero Qty", 10.0, 0)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "Zero quantity should be rejected");
    }

    // ========================================================================
    // 26. 负数量 — 当前被接受 (导致负总额)
    // ========================================================================

    #[test]
    fn test_add_items_with_negative_quantity_rejected() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-neg-qty".to_string()),
                table_name: Some("Table Neg Qty".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Negative Qty", 10.0, -3)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "Negative quantity should be rejected");
    }

    // ========================================================================
    // 27. i32::MAX 数量 — Decimal 可以处理
    // ========================================================================

    #[test]
    fn test_add_items_with_i32_max_quantity_rejected() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-maxqty".to_string()),
                table_name: Some("Table Max Qty".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Max Qty", 0.01, i32::MAX)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "i32::MAX quantity exceeds max (9999), should be rejected");
    }

    #[test]
    fn test_add_items_with_max_allowed_quantity() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-max-allowed-qty",
            vec![simple_item("product:p1", "Max Allowed", 0.01, 9999)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items[0].quantity, 9999);
        // 0.01 * 9999 = 99.99
        assert_eq!(snapshot.subtotal, 99.99);
    }

    // ========================================================================
    // 28. 折扣超过 100% — unit_price clamp 到 0
    // ========================================================================

    #[test]
    fn test_add_items_with_discount_over_100_percent() {
        let manager = create_test_manager();

        // Open table
        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-over-disc".to_string()),
                table_name: Some("Table Over Discount".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // Add item with 200% discount
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Over Discounted".to_string(),
                    price: 100.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: None,
                    selected_specification: None,
                    manual_discount_percent: Some(200.0),
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "200% discount should be rejected (max 100%)");
    }

    // ========================================================================
    // 29. 负折扣 — 当前被接受 (相当于加价)
    // ========================================================================

    #[test]
    fn test_add_items_with_negative_discount_acts_as_markup() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-neg-disc".to_string()),
                table_name: Some("Table Neg Discount".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Neg Discount Item".to_string(),
                    price: 100.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: None,
                    selected_specification: None,
                    manual_discount_percent: Some(-50.0), // -50% = +50%
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "Negative discount should be rejected (min 0%)");
    }

    // ========================================================================
    // 30. 支付 NaN 金额 — 当前被 <= 0.0 检查通过 (NaN 比较特殊)
    // ========================================================================

    #[test]
    fn test_add_payment_with_nan_amount_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-nan-pay",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: f64::NAN,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "NaN payment amount should be rejected");
    }

    // ========================================================================
    // 31. 支付 Infinity 金额 — 同样绕过 <= 0.0 检查
    // ========================================================================

    #[test]
    fn test_add_payment_with_infinity_amount_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-inf-pay",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: f64::INFINITY,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "Infinity payment amount should be rejected");
    }

    // ========================================================================
    // 32. 支付 f64::MAX — 绕过检查，但 Decimal 转换为 0
    // ========================================================================

    #[test]
    fn test_add_payment_with_f64_max_amount_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-maxpay",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: f64::MAX,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "f64::MAX payment should be rejected (exceeds max)");
    }

    // ========================================================================
    // 33. 多个极端商品叠加后完成订单
    // ========================================================================

    #[test]
    fn test_multiple_edge_items_then_complete() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-multi-edge".to_string()),
                table_name: Some("Table Multi Edge".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // 正常商品 + 零价格商品
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![
                    simple_item("product:p1", "Normal", 25.50, 2),
                    simple_item("product:p2", "Free", 0.0, 1),
                    simple_item("product:p3", "Cheap", 0.01, 100),
                ],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 25.50*2 + 0*1 + 0.01*100 = 51.0 + 0 + 1.0 = 52.0
        assert_eq!(snapshot.subtotal, 52.0);

        // 支付并完成
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 52.0,
                    tendered: Some(60.0),
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert_eq!(snapshot.payments[0].change, Some(8.0)); // 60 - 52 = 8
    }

    // ========================================================================
    // 34. 带选项价格修改器的边界测试
    // ========================================================================

    #[test]
    fn test_add_items_with_option_price_modifiers() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-opts".to_string()),
                table_name: Some("Table Opts".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Pizza".to_string(),
                    price: 12.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: Some(vec![
                        shared::order::ItemOption {
                            attribute_id: "attr:size".to_string(),
                            attribute_name: "Size".to_string(),
                            option_idx: 2,
                            option_name: "Large".to_string(),
                            price_modifier: Some(3.0), // +3
                            quantity: 1,
                        },
                        shared::order::ItemOption {
                            attribute_id: "attr:topping".to_string(),
                            attribute_name: "Topping".to_string(),
                            option_idx: 0,
                            option_name: "Extra Cheese".to_string(),
                            price_modifier: Some(1.50), // +1.50
                            quantity: 1,
                        },
                    ]),
                    selected_specification: None,
                    manual_discount_percent: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // reducer: item_final = base(12) + options(3+1.50) = 16.50
        // money: original_price=12.0, options=4.50, base_with_options=16.50
        //   unit_price=16.50, line_total=16.50
        assert_eq!(snapshot.subtotal, 16.5);
    }

    // ========================================================================
    // 35. 选项修改器为负值 — 当前被接受
    // ========================================================================

    #[test]
    fn test_add_items_with_negative_option_modifier() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-neg-opt".to_string()),
                table_name: Some("Table Neg Opt".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Special".to_string(),
                    price: 10.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: Some(vec![shared::order::ItemOption {
                        attribute_id: "attr:mod".to_string(),
                        attribute_name: "Mod".to_string(),
                        option_idx: 0,
                        option_name: "Smaller".to_string(),
                        price_modifier: Some(-15.0), // -15 使总价变负
                        quantity: 1,
                    }]),
                    selected_specification: None,
                    manual_discount_percent: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        let resp = manager.execute_command(add_cmd);
        // 负的 price_modifier 是被允许的 (比如更小的规格减价)
        // 但不能超过 MAX_PRICE 的绝对值
        assert!(resp.success, "Negative option modifier within bounds is allowed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // reducer: base=10+(-15)=-5, item_final=max(0,-5)=0
        // money: base_price=0, options=-15, base_with_options=-15 → clamped to 0
        assert_eq!(snapshot.subtotal, 0.0, "Negative modifier can reduce price to 0");
    }

    // ========================================================================
    // 37. 现金支付 tendered < amount 应被拒绝
    // ========================================================================

    #[test]
    fn test_add_cash_payment_tendered_less_than_amount_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-short-tender",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: Some(5.0), // 给了 5 块，要付 10 块
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "Tendered less than amount should be rejected");
    }

    // ========================================================================
    // 38. 折扣 + 附加费 + 选项叠加后精度测试
    // ========================================================================

    #[test]
    fn test_discount_surcharge_options_combined_precision() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-combo".to_string()),
                table_name: Some("Table Combo".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Combo Item".to_string(),
                    price: 33.33,
                    original_price: None,
                    quantity: 3,
                    selected_options: Some(vec![shared::order::ItemOption {
                        attribute_id: "attr:size".to_string(),
                        attribute_name: "Size".to_string(),
                        option_idx: 1,
                        option_name: "Large".to_string(),
                        price_modifier: Some(1.67),
                        quantity: 1,
                    }]),
                    selected_specification: None,
                    manual_discount_percent: Some(10.0), // 10% off
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // reducer: base=33.33+1.67=35.0, discount=3.5, item_final=31.5
        // money: original_price=33.33, options=1.67, base_with_options=35.0
        //   manual_discount=35.0*10/100=3.5
        //   unit_price=35.0-3.5=31.5
        //   line_total=31.5*3=94.5
        assert_eq!(snapshot.subtotal, 94.5);
    }

    // ========================================================================
    // 39. 支付 NaN 后尝试完成订单 — 应该失败
    // ========================================================================

    #[test]
    fn test_nan_payment_then_complete_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-nan-complete",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // NaN payment — 被输入验证拒绝
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: f64::NAN,
                    tendered: None,
                    note: None,
                },
            },
        );
        let pay_resp = manager.execute_command(pay_cmd);
        assert!(!pay_resp.success, "NaN payment should be rejected by validation");

        // 尝试完成 — 应该失败因为没有成功的支付
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(!resp.success, "Should fail: no payment was recorded");
    }

    // ========================================================================
    // 40. 快照重建一致性 — 带边界值
    // ========================================================================

    #[test]
    fn test_rebuild_snapshot_with_edge_values() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-rebuild-edge".to_string()),
                table_name: Some("Table Rebuild Edge".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // 添加零价格商品
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![
                    simple_item("product:p1", "Free", 0.0, 5),
                    simple_item("product:p2", "Penny", 0.01, 99),
                ],
            },
        );
        manager.execute_command(add_cmd);

        let stored = manager.get_snapshot(&order_id).unwrap().unwrap();
        let rebuilt = manager.rebuild_snapshot(&order_id).unwrap();

        assert_eq!(stored.subtotal, rebuilt.subtotal);
        assert_eq!(stored.total, rebuilt.total);
        assert_eq!(stored.state_checksum, rebuilt.state_checksum);
        // 0*5 + 0.01*99 = 0.99
        assert_eq!(stored.subtotal, 0.99);
    }

    // ========================================================================
    // 41. 批量小金额累加精度
    // ========================================================================

    #[test]
    fn test_many_small_amounts_precision() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-small-amounts".to_string()),
                table_name: Some("Table Small".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // 添加 10 次，每次 1 个 0.1 的商品
        for i in 0..10 {
            let add_cmd = OrderCommand::new(
                "op-1".to_string(),
                "Test Operator".to_string(),
                OrderCommandPayload::AddItems {
                    order_id: order_id.clone(),
                    items: vec![CartItemInput {
                        product_id: format!("product:p{}", i),
                        name: format!("Item {}", i),
                        price: 0.1,
                        original_price: None,
                        quantity: 1,
                        selected_options: None,
                        selected_specification: None,
                        manual_discount_percent: None,
                        note: None,
                        authorizer_id: None,
                        authorizer_name: None,
                    }],
                },
            );
            manager.execute_command(add_cmd);
        }

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 0.1 * 10 = 1.0 (使用 Decimal 精确计算)
        assert_eq!(snapshot.subtotal, 1.0, "10 x 0.1 should be exactly 1.0");
        assert_eq!(snapshot.total, 1.0);
    }

    // ========================================================================
    // 42. NaN tendered — 对应 amount 为正值
    // ========================================================================

    #[test]
    fn test_add_cash_payment_nan_tendered() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-nan-tender",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: Some(f64::NAN), // NaN tendered
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        // to_decimal(NaN) = 0, to_decimal(10.0) - 0.01 = 9.99
        // 0 < 9.99 → tendered 不足被拒绝
        assert!(!resp.success, "NaN tendered should fail: Decimal(0) < 9.99");
    }

    // ========================================================================
    // 43. Moved/Merged 状态的订单不能添加支付
    // ========================================================================

    #[test]
    fn test_add_payment_to_moved_order_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-moved-pay",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // Move order
        let move_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: "T-moved-pay-2".to_string(),
                target_table_name: "Table 2".to_string(),
                target_zone_id: None,
                target_zone_name: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(move_cmd);

        // MoveOrder 在当前实现中不改变订单状态为 Moved（它只是移动桌台），
        // 而是保持 Active。验证移桌后仍然可以支付。
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(resp.success, "Moved order should still accept payments (status stays Active)");
    }

    // ========================================================================
    // 44. 极小金额差异 — 支付容差边界
    // ========================================================================

    #[test]
    fn test_payment_tolerance_boundary() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-tolerance",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // 支付 9.99 — 差 0.01，在容差内
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: 9.99,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success, "9.99 should be sufficient for 10.0 (within 0.01 tolerance)");
    }

    #[test]
    fn test_payment_below_tolerance_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-below-tol",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // 支付 9.98 — 差 0.02，超出容差
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: 9.98,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(!resp.success, "9.98 should be insufficient for 10.0 (outside 0.01 tolerance)");
    }

    // ========================================================================
    // 41. 多选项 + 手动折扣 + 规则字段: 端到端精度验证 (无双重计算)
    // ========================================================================

    #[test]
    fn test_options_discount_rule_fields_no_double_counting() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-no-double".to_string()),
                table_name: Some("Table No Double".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // Item: price=20.0, options=+3.0+2.0=5.0, discount=10%
        // Expected: base_with_options=25.0, discount=2.5, unit_price=22.5
        // qty=2 → subtotal=45.0
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Steak".to_string(),
                    price: 20.0,
                    original_price: None,
                    quantity: 2,
                    selected_options: Some(vec![
                        shared::order::ItemOption {
                            attribute_id: "attr:sauce".to_string(),
                            attribute_name: "Sauce".to_string(),
                            option_idx: 0,
                            option_name: "BBQ".to_string(),
                            price_modifier: Some(3.0),
                            quantity: 1,
                        },
                        shared::order::ItemOption {
                            attribute_id: "attr:side".to_string(),
                            attribute_name: "Side".to_string(),
                            option_idx: 1,
                            option_name: "Fries".to_string(),
                            price_modifier: Some(2.0),
                            quantity: 1,
                        },
                    ]),
                    selected_specification: None,
                    manual_discount_percent: Some(10.0),
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item = &snapshot.items[0];

        // original_price should be set to the input price (spec price)
        assert_eq!(item.original_price, Some(20.0), "original_price = input price");

        // unit_price: base(20)+options(5)=25, discount=25*10%=2.5, unit=22.5
        assert_eq!(item.unit_price, Some(22.5), "unit_price = 22.5 (no double counting)");

        // subtotal = 22.5 * 2 = 45.0
        assert_eq!(snapshot.subtotal, 45.0, "subtotal = 45.0");
        assert_eq!(snapshot.total, 45.0, "total = 45.0 (no tax)");
    }

    // ========================================================================
    // 42. ModifyItem 后 unit_price 一致性
    // ========================================================================

    #[test]
    fn test_modify_item_unit_price_consistency() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-mod-cons".to_string()),
                table_name: Some("Table Mod Consistency".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // Add item: price=15.0, options=+2.5, no discount, qty=3
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "product:p1".to_string(),
                    name: "Pasta".to_string(),
                    price: 15.0,
                    original_price: None,
                    quantity: 3,
                    selected_options: Some(vec![shared::order::ItemOption {
                        attribute_id: "attr:cheese".to_string(),
                        attribute_name: "Cheese".to_string(),
                        option_idx: 0,
                        option_name: "Extra".to_string(),
                        price_modifier: Some(2.5),
                        quantity: 1,
                    }]),
                    selected_specification: None,
                    manual_discount_percent: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success);

        let snapshot_before = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot_before.items[0].instance_id.clone();
        // unit_price = 15 + 2.5 = 17.5, subtotal = 17.5 * 3 = 52.5
        assert_eq!(snapshot_before.items[0].unit_price, Some(17.5));
        assert_eq!(snapshot_before.subtotal, 52.5);

        // Modify: add 20% discount
        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    price: None,
                    quantity: None,
                    manual_discount_percent: Some(20.0),
                    note: None,
                    selected_options: None,
                    selected_specification: None,
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(resp.success, "ModifyItem should succeed");

        let snapshot_after = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item = &snapshot_after.items[0];

        // original_price should still be 15.0
        assert_eq!(item.original_price, Some(15.0), "original_price unchanged after modify");

        // unit_price: base(15)+options(2.5)=17.5, discount=17.5*20%=3.5, unit=14.0
        assert_eq!(item.unit_price, Some(14.0), "unit_price after 20% discount");

        // subtotal = 14.0 * 3 = 42.0
        assert_eq!(snapshot_after.subtotal, 42.0, "subtotal after modify");
        assert_eq!(snapshot_after.total, 42.0, "total after modify");
    }

    // ========================================================================
    // ========================================================================
    //  恶意数据防御 + 死胡同预防测试
    // ========================================================================
    // ========================================================================

    // ========================================================================
    // 状态守卫: Voided 订单不可操作
    // ========================================================================

    #[test]
    fn test_add_items_to_voided_order_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-void-add",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // Void
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(void_cmd);

        // Try to add items
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p2", "Tea", 5.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "Should not add items to voided order");
    }

    #[test]
    fn test_add_payment_to_voided_order_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-void-pay",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(void_cmd);

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "Should not add payment to voided order");
    }

    #[test]
    fn test_complete_voided_order_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(&manager, "T-void-complete", vec![]);

        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(void_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(!resp.success, "Should not complete a voided order");
    }

    // ========================================================================
    // 状态守卫: Completed 订单不可 void
    // ========================================================================

    #[test]
    fn test_void_completed_order_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-comp-void",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // Pay + complete
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd);

        // Try to void
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(void_cmd);
        assert!(!resp.success, "Should not void a completed order");
    }

    #[test]
    fn test_double_complete_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-dbl-complete",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp1 = manager.execute_command(complete_cmd);
        assert!(resp1.success);

        let complete_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp2 = manager.execute_command(complete_cmd2);
        assert!(!resp2.success, "Double complete should fail");
    }

    // ========================================================================
    // 恶意 ModifyItem 数据
    // ========================================================================

    #[test]
    fn test_modify_item_nan_price_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-mod-nan",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    price: Some(f64::NAN),
                    quantity: None,
                    manual_discount_percent: None,
                    note: None,
                    selected_options: None,
                    selected_specification: None,
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(!resp.success, "ModifyItem with NaN price should be rejected");
    }

    #[test]
    fn test_modify_item_negative_price_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-mod-neg",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    price: Some(-50.0),
                    quantity: None,
                    manual_discount_percent: None,
                    note: None,
                    selected_options: None,
                    selected_specification: None,
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(!resp.success, "ModifyItem with negative price should be rejected");
    }

    #[test]
    fn test_modify_item_nan_discount_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-mod-nan-disc",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    price: None,
                    quantity: None,
                    manual_discount_percent: Some(f64::NAN),
                    note: None,
                    selected_options: None,
                    selected_specification: None,
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(!resp.success, "ModifyItem with NaN discount should be rejected");
    }

    #[test]
    fn test_modify_item_discount_over_100_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-mod-disc-150",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    price: None,
                    quantity: None,
                    manual_discount_percent: Some(150.0),
                    note: None,
                    selected_options: None,
                    selected_specification: None,
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(!resp.success, "ModifyItem with 150% discount should be rejected");
    }

    #[test]
    fn test_modify_item_zero_quantity_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-mod-zeroq",
            vec![simple_item("product:p1", "Coffee", 10.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    price: None,
                    quantity: Some(0),
                    manual_discount_percent: None,
                    note: None,
                    selected_options: None,
                    selected_specification: None,
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(!resp.success, "ModifyItem with quantity=0 should be rejected");
    }

    // ========================================================================
    // 空 items 数组攻击
    // ========================================================================

    #[test]
    fn test_add_empty_items_array() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(&manager, "T-empty-arr", vec![]);

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![], // Empty array
            },
        );
        let _resp = manager.execute_command(add_cmd);
        // 即使 AddItems 允许空数组（当前行为），订单不应进入不一致状态
        // 记录当前行为，不管成功与否，订单仍可继续操作
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Active, "Order should remain Active");
        assert_eq!(snapshot.items.len(), 0, "No items should be added");

        // 验证可以继续添加正常商品 (不进入死胡同)
        let add_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Coffee", 10.0, 1)],
            },
        );
        let resp2 = manager.execute_command(add_cmd2);
        assert!(resp2.success, "Should be able to add items after empty array");
    }

    // ========================================================================
    // 合并操作: 无效目标
    // ========================================================================

    #[test]
    fn test_merge_voided_source_fails() {
        let manager = create_test_manager();
        let source_id = open_table_with_items(&manager, "T-merge-vs", vec![]);
        let target_id = open_table_with_items(&manager, "T-merge-vt", vec![]);

        // Void source
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: source_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(void_cmd);

        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: source_id.clone(),
                target_order_id: target_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(!resp.success, "Should not merge a voided source order");
    }

    #[test]
    fn test_merge_into_voided_target_fails() {
        let manager = create_test_manager();
        let source_id = open_table_with_items(&manager, "T-merge-ts", vec![]);
        let target_id = open_table_with_items(&manager, "T-merge-tt", vec![]);

        // Void target
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: target_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(void_cmd);

        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: source_id.clone(),
                target_order_id: target_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(!resp.success, "Should not merge into a voided target order");
    }

    #[test]
    fn test_merge_self_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(&manager, "T-self-merge", vec![]);

        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: order_id.clone(),
                target_order_id: order_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(!resp.success, "Should not merge order with itself");
    }

    // ========================================================================
    // AA Split 恶意数据
    // ========================================================================

    #[test]
    fn test_aa_split_zero_total_shares_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-aa-zero",
            vec![simple_item("product:p1", "Coffee", 30.0, 1)],
        );

        let cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::StartAaSplit {
                order_id: order_id.clone(),
                total_shares: 0, // Invalid
                shares: 0,
                payment_method: "CASH".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(cmd);
        assert!(!resp.success, "AA split with 0 total shares should fail");
    }

    #[test]
    fn test_aa_split_one_share_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-aa-one",
            vec![simple_item("product:p1", "Coffee", 30.0, 1)],
        );

        let cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::StartAaSplit {
                order_id: order_id.clone(),
                total_shares: 1, // Must be >= 2
                shares: 1,
                payment_method: "CASH".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(cmd);
        assert!(!resp.success, "AA split with 1 total share should fail (need >= 2)");
    }

    #[test]
    fn test_aa_split_shares_exceed_total_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-aa-exceed",
            vec![simple_item("product:p1", "Coffee", 30.0, 1)],
        );

        let cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::StartAaSplit {
                order_id: order_id.clone(),
                total_shares: 3,
                shares: 5, // More than total
                payment_method: "CASH".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(cmd);
        assert!(!resp.success, "AA split shares > total_shares should fail");
    }

    #[test]
    fn test_pay_aa_split_exceed_remaining_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-aa-overpay",
            vec![simple_item("product:p1", "Coffee", 30.0, 1)],
        );

        // Start AA: 3 shares, pay 2
        let start_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::StartAaSplit {
                order_id: order_id.clone(),
                total_shares: 3,
                shares: 2,
                payment_method: "CASH".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(start_cmd);
        assert!(resp.success);

        // Try to pay 3 more shares (only 1 remaining)
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::PayAaSplit {
                order_id: order_id.clone(),
                shares: 3,
                payment_method: "CASH".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "Pay AA split with shares > remaining should fail");
    }

    // ========================================================================
    // 取消已取消的支付
    // ========================================================================

    #[test]
    fn test_cancel_already_cancelled_payment_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-dbl-cancel",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // Pay
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CARD".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let payment_id = snapshot.payments[0].payment_id.clone();

        // Cancel once
        let cancel1 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CancelPayment {
                order_id: order_id.clone(),
                payment_id: payment_id.clone(),
                reason: Some("mistake".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp1 = manager.execute_command(cancel1);
        assert!(resp1.success);

        // Cancel again (should fail)
        let cancel2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CancelPayment {
                order_id: order_id.clone(),
                payment_id,
                reason: Some("again".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp2 = manager.execute_command(cancel2);
        assert!(!resp2.success, "Should not cancel an already-cancelled payment");
    }

    // ========================================================================
    // 移桌到已占用桌台
    // ========================================================================

    #[test]
    fn test_move_to_occupied_table_fails() {
        let manager = create_test_manager();
        let _order1 = open_table_with_items(
            &manager,
            "T-occ-1",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );
        let order2 = open_table_with_items(
            &manager,
            "T-occ-2",
            vec![simple_item("product:p2", "Tea", 5.0, 1)],
        );

        // Move order2 to T-occ-1 (occupied)
        let move_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order2.clone(),
                target_table_id: "T-occ-1".to_string(),
                target_table_name: "Table 1".to_string(),
                target_zone_id: None,
                target_zone_name: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(!resp.success, "Should not move to an occupied table");
    }

    // ========================================================================
    // ModifyItem 对已完成/已取消订单
    // ========================================================================

    #[test]
    fn test_modify_item_on_completed_order_fails() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-mod-comp",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // Pay + complete
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd);

        // Try to modify
        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    price: Some(999.0),
                    quantity: None,
                    manual_discount_percent: None,
                    note: None,
                    selected_options: None,
                    selected_specification: None,
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(!resp.success, "Should not modify items on completed order");
    }

    // ========================================================================
    // 支付负金额
    // ========================================================================

    #[test]
    fn test_add_payment_negative_amount_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-neg-pay-amt",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: -10.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "Negative payment amount should be rejected");
    }

    #[test]
    fn test_add_payment_zero_amount_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            "T-zero-pay-amt",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 0.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "Zero payment amount should be rejected");
    }

    // ========================================================================
    // 规则快照持久化测试
    // ========================================================================

    fn create_test_rule(name: &str) -> PriceRule {
        use crate::db::models::price_rule::{AdjustmentType, ProductScope, RuleType};
        PriceRule {
            id: None,
            name: name.to_string(),
            display_name: name.to_string(),
            receipt_name: name.to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: "zone:all".to_string(),
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10.0,
            is_stackable: false,
            is_exclusive: false,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 0,
        }
    }

    #[test]
    fn test_cache_rules_persists_to_redb() {
        let manager = create_test_manager();
        let order_id = "order-persist-1";

        let rules = vec![create_test_rule("Lunch Special"), create_test_rule("VIP")];
        manager.cache_rules(order_id, rules);

        // 内存缓存应该有
        let cached = manager.get_cached_rules(order_id);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 2);

        // redb 也应该有
        let persisted = manager.storage().get_rule_snapshot(order_id).unwrap();
        assert!(persisted.is_some());
        assert_eq!(persisted.unwrap().len(), 2);
    }

    #[test]
    fn test_remove_cached_rules_cleans_redb() {
        let manager = create_test_manager();
        let order_id = "order-remove-1";

        manager.cache_rules(order_id, vec![create_test_rule("Rule")]);
        assert!(manager.get_cached_rules(order_id).is_some());
        assert!(manager.storage().get_rule_snapshot(order_id).unwrap().is_some());

        // 清除
        manager.remove_cached_rules(order_id);

        // 内存和 redb 都应该被清除
        assert!(manager.get_cached_rules(order_id).is_none());
        assert!(manager.storage().get_rule_snapshot(order_id).unwrap().is_none());
    }

    #[test]
    fn test_restore_rule_snapshots_from_redb() {
        let storage = OrderStorage::open_in_memory().unwrap();

        // 注册为活跃订单（模拟正常开台后的状态）
        {
            let txn = storage.begin_write().unwrap();
            storage.mark_order_active(&txn, "order-a").unwrap();
            storage.mark_order_active(&txn, "order-b").unwrap();
            txn.commit().unwrap();
        }

        // 写入规则快照（模拟上次运行遗留的快照）
        storage.store_rule_snapshot("order-a", &vec![create_test_rule("Rule A")]).unwrap();
        storage.store_rule_snapshot("order-b", &vec![create_test_rule("Rule B1"), create_test_rule("Rule B2")]).unwrap();

        // 创建新 manager（模拟重启，内存缓存为空）
        let manager = OrdersManager::with_storage(storage);
        assert!(manager.get_cached_rules("order-a").is_none());
        assert!(manager.get_cached_rules("order-b").is_none());

        // 恢复
        let count = manager.restore_rule_snapshots_from_redb();
        assert_eq!(count, 2);

        // 内存缓存应该有了
        let rules_a = manager.get_cached_rules("order-a").unwrap();
        assert_eq!(rules_a.len(), 1);
        assert_eq!(rules_a[0].name, "Rule A");

        let rules_b = manager.get_cached_rules("order-b").unwrap();
        assert_eq!(rules_b.len(), 2);
    }

    #[test]
    fn test_restore_rule_snapshots_cleans_orphans() {
        let storage = OrderStorage::open_in_memory().unwrap();

        // 只注册 order-a 为活跃，order-orphan 不注册（模拟崩溃后的孤儿快照）
        {
            let txn = storage.begin_write().unwrap();
            storage.mark_order_active(&txn, "order-a").unwrap();
            txn.commit().unwrap();
        }

        storage.store_rule_snapshot("order-a", &vec![create_test_rule("Rule A")]).unwrap();
        storage.store_rule_snapshot("order-orphan", &vec![create_test_rule("Orphan Rule")]).unwrap();

        let manager = OrdersManager::with_storage(storage);
        let count = manager.restore_rule_snapshots_from_redb();

        // 只恢复了活跃订单的规则
        assert_eq!(count, 1);
        assert!(manager.get_cached_rules("order-a").is_some());
        assert!(manager.get_cached_rules("order-orphan").is_none());

        // 孤儿快照应该已从 redb 中清除
        assert!(manager.storage().get_rule_snapshot("order-orphan").unwrap().is_none());
    }

    #[test]
    fn test_complete_order_cleans_rules() {
        let manager = create_test_manager();

        // 开台
        let open_cmd = create_open_table_cmd("op-1");
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // 缓存规则
        manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
        assert!(manager.get_cached_rules(&order_id).is_some());

        // 加菜
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Coffee", 10.0, 1)],
            },
        );
        manager.execute_command(add_cmd);

        // 支付
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 10.0,
                    tendered: Some(10.0),
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd);

        // 完成订单
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd);

        // 规则缓存和 redb 快照都应该被清除
        assert!(manager.get_cached_rules(&order_id).is_none());
        assert!(manager.storage().get_rule_snapshot(&order_id).unwrap().is_none());
    }

    #[test]
    fn test_void_order_cleans_rules() {
        let manager = create_test_manager();

        // 开台
        let open_cmd = create_open_table_cmd("op-1");
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // 缓存规则
        manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
        assert!(manager.get_cached_rules(&order_id).is_some());

        // 作废订单
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(void_cmd);

        // 规则缓存和 redb 快照都应该被清除
        assert!(manager.get_cached_rules(&order_id).is_none());
        assert!(manager.storage().get_rule_snapshot(&order_id).unwrap().is_none());
    }

    #[test]
    fn test_move_order_cleans_rules() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-rule-move",
            vec![simple_item("product:p1", "Coffee", 5.0, 1)],
        );

        // 缓存规则
        manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
        assert!(manager.get_cached_rules(&order_id).is_some());
        assert!(manager.storage().get_rule_snapshot(&order_id).unwrap().is_some());

        // 换桌
        let move_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: "T-rule-move-2".to_string(),
                target_table_name: "Table T-rule-move-2".to_string(),
                target_zone_id: Some("zone:z2".to_string()),
                target_zone_name: Some("Zone B".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(resp.success);

        // 源订单的规则缓存和 redb 快照都应该被清除
        assert!(manager.get_cached_rules(&order_id).is_none());
        assert!(manager.storage().get_rule_snapshot(&order_id).unwrap().is_none());
    }

    #[test]
    fn test_merge_orders_cleans_source_rules() {
        let manager = create_test_manager();

        // 源订单
        let source_id = open_table_with_items(
            &manager,
            "T-rule-merge-src",
            vec![simple_item("product:p1", "Coffee", 10.0, 1)],
        );

        // 目标订单
        let target_id = open_table_with_items(
            &manager,
            "T-rule-merge-tgt",
            vec![simple_item("product:p2", "Tea", 8.0, 1)],
        );

        // 给源订单缓存规则
        manager.cache_rules(&source_id, vec![create_test_rule("SourceRule")]);
        assert!(manager.get_cached_rules(&source_id).is_some());
        assert!(manager.storage().get_rule_snapshot(&source_id).unwrap().is_some());

        // 合并 source → target
        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: source_id.clone(),
                target_order_id: target_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(resp.success);

        // 源订单的规则缓存和 redb 快照都应该被清除
        assert!(manager.get_cached_rules(&source_id).is_none());
        assert!(manager.storage().get_rule_snapshot(&source_id).unwrap().is_none());
    }

    // ========================================================================
    // ========================================================================
    //  新增测试工具函数
    // ========================================================================
    // ========================================================================

    /// 创建带选项的商品
    fn item_with_options(
        product_id: &str,
        name: &str,
        price: f64,
        quantity: i32,
        options: Vec<shared::order::ItemOption>,
    ) -> CartItemInput {
        CartItemInput {
            product_id: product_id.to_string(),
            name: name.to_string(),
            price,
            original_price: None,
            quantity,
            selected_options: Some(options),
            selected_specification: None,
            manual_discount_percent: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    /// 创建带折扣的商品
    fn item_with_discount(
        product_id: &str,
        name: &str,
        price: f64,
        quantity: i32,
        discount_percent: f64,
    ) -> CartItemInput {
        CartItemInput {
            product_id: product_id.to_string(),
            name: name.to_string(),
            price,
            original_price: None,
            quantity,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(discount_percent),
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    /// 快速支付
    fn pay_order(manager: &OrdersManager, order_id: &str, amount: f64, method: &str) -> CommandResponse {
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.to_string(),
                payment: PaymentInput {
                    method: method.to_string(),
                    amount,
                    tendered: if method == "CASH" { Some(amount) } else { None },
                    note: None,
                },
            },
        );
        manager.execute_command(pay_cmd)
    }

    /// 快速完成订单
    fn complete_order(manager: &OrdersManager, order_id: &str) -> CommandResponse {
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.to_string(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd)
    }

    /// 快速作废订单
    fn void_order_helper(manager: &OrdersManager, order_id: &str, void_type: VoidType) -> CommandResponse {
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.to_string(),
                void_type,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(void_cmd)
    }

    /// 断言订单状态
    fn assert_order_status(manager: &OrdersManager, order_id: &str, expected: OrderStatus) {
        let snapshot = manager.get_snapshot(order_id).unwrap().unwrap();
        assert_eq!(
            snapshot.status, expected,
            "Expected order status {:?}, got {:?}",
            expected, snapshot.status
        );
    }

    /// 断言订单金额 (使用近似比较)
    fn assert_order_total(manager: &OrdersManager, order_id: &str, expected: f64) {
        let snapshot = manager.get_snapshot(order_id).unwrap().unwrap();
        let diff = (snapshot.total - expected).abs();
        assert!(
            diff < 0.01,
            "Expected order total {}, got {}",
            expected, snapshot.total
        );
    }

    /// 打开零售订单
    fn open_retail_order(manager: &OrdersManager) -> String {
        let cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: None,
                table_name: None,
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: true,
            },
        );
        let resp = manager.execute_command(cmd);
        assert!(resp.success, "Failed to open retail order");
        resp.order_id.unwrap()
    }

    // ========================================================================
    // ========================================================================
    //  P0: 核心业务流程测试
    // ========================================================================
    // ========================================================================

    // ------------------------------------------------------------------------
    // P0.1: 完整堂食订单生命周期
    // OpenTable → AddItems(3种) → ModifyItem → AddPayment → CompleteOrder
    // ------------------------------------------------------------------------
    #[test]
    fn test_complete_dine_in_flow() {
        let manager = create_test_manager();

        // 1. 开台
        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-dine-flow".to_string()),
                table_name: Some("Table Dine Flow".to_string()),
                zone_id: Some("zone:z1".to_string()),
                zone_name: Some("Zone A".to_string()),
                guest_count: 4,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        assert!(resp.success, "OpenTable should succeed");
        let order_id = resp.order_id.unwrap();

        // 验证开台状态
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Active);
        assert_eq!(snapshot.guest_count, 4);
        assert!(!snapshot.receipt_number.is_empty());

        // 2. 添加 3 种商品
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![
                    simple_item("product:coffee", "Coffee", 5.0, 2),      // 10.0
                    simple_item("product:tea", "Tea", 3.0, 3),            // 9.0
                    simple_item("product:cake", "Cake", 12.50, 1),        // 12.50
                ],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success, "AddItems should succeed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items.len(), 3);
        assert_eq!(snapshot.subtotal, 31.5); // 10 + 9 + 12.5

        // 3. ModifyItem: 减少 Tea 数量 3 → 2
        let tea_instance_id = snapshot.items.iter()
            .find(|i| i.name == "Tea")
            .unwrap()
            .instance_id.clone();

        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id: tea_instance_id,
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    quantity: Some(2),
                    ..Default::default()
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(resp.success, "ModifyItem should succeed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 新总额: 10 + 6 + 12.5 = 28.5
        assert_eq!(snapshot.subtotal, 28.5);
        assert_eq!(snapshot.total, 28.5);

        // 4. 全额支付
        let pay_resp = pay_order(&manager, &order_id, 28.5, "CARD");
        assert!(pay_resp.success, "Payment should succeed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 28.5);

        // 5. 完成订单
        let complete_resp = complete_order(&manager, &order_id);
        assert!(complete_resp.success, "CompleteOrder should succeed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert!(snapshot.end_time.is_some());
    }

    // ------------------------------------------------------------------------
    // P0.2: 完整零售订单流程 (带 queue_number)
    // ------------------------------------------------------------------------
    #[test]
    fn test_complete_retail_flow_with_queue_number() {
        let manager = create_test_manager();

        // 1. 开零售订单
        let order_id = open_retail_order(&manager);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.is_retail);
        assert!(snapshot.queue_number.is_some(), "Retail order should have queue_number");
        assert!(snapshot.table_id.is_none(), "Retail order should have no table_id");

        // 2. 添加商品
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p1", "Item", 15.0, 2)],
            },
        );
        manager.execute_command(add_cmd);

        // 3. 支付
        pay_order(&manager, &order_id, 30.0, "CASH");

        // 4. 完成 (指定服务类型为外带)
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::Takeout),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert_eq!(snapshot.service_type, Some(ServiceType::Takeout));
    }

    // ------------------------------------------------------------------------
    // P0.3: VoidOrder 损失结算
    // ------------------------------------------------------------------------
    #[test]
    fn test_void_order_loss_settlement() {
        let manager = create_test_manager();

        // 开台 + 添加商品
        let order_id = open_table_with_items(
            &manager,
            "T-loss",
            vec![simple_item("product:p1", "Expensive Item", 100.0, 1)],
        );

        // 损失结算作废
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::LossSettled,
                loss_reason: Some(shared::order::LossReason::CustomerFled),
                loss_amount: Some(100.0),
                note: Some("Customer fled without paying".to_string()),
                authorizer_id: Some("auth:1".to_string()),
                authorizer_name: Some("Manager".to_string()),
            },
        );
        let resp = manager.execute_command(void_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Void);
        assert_eq!(snapshot.void_type, Some(VoidType::LossSettled));
        assert_eq!(snapshot.loss_reason, Some(shared::order::LossReason::CustomerFled));
        assert_eq!(snapshot.loss_amount, Some(100.0));
    }

    // ------------------------------------------------------------------------
    // P0.4: 多次菜品分单后完成
    // ------------------------------------------------------------------------
    #[test]
    fn test_split_by_items_multiple_then_complete() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-multi-split",
            vec![
                simple_item("product:p1", "Item A", 20.0, 2),  // 40
                simple_item("product:p2", "Item B", 15.0, 2),  // 30
                simple_item("product:p3", "Item C", 10.0, 1),  // 10
            ],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 80.0);
        let item_a_id = snapshot.items.iter().find(|i| i.name == "Item A").unwrap().instance_id.clone();
        let item_b_id = snapshot.items.iter().find(|i| i.name == "Item B").unwrap().instance_id.clone();

        // 第一次分单: 支付 Item A 的 1 个 (20.0)
        let split_cmd1 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByItems {
                order_id: order_id.clone(),
                payment_method: "CASH".to_string(),
                items: vec![shared::order::SplitItem {
                    instance_id: item_a_id.clone(),
                    name: "Item A".to_string(),
                    quantity: 1,
                    unit_price: 20.0,
                }],
                tendered: Some(20.0),
            },
        );
        let resp = manager.execute_command(split_cmd1);
        assert!(resp.success, "First split should succeed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 20.0);
        assert_eq!(snapshot.paid_item_quantities.get(&item_a_id), Some(&1));

        // 第二次分单: 支付 Item B 的 2 个 (30.0)
        let split_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByItems {
                order_id: order_id.clone(),
                payment_method: "CARD".to_string(),
                items: vec![shared::order::SplitItem {
                    instance_id: item_b_id.clone(),
                    name: "Item B".to_string(),
                    quantity: 2,
                    unit_price: 15.0,
                }],
                tendered: None,
            },
        );
        let resp = manager.execute_command(split_cmd2);
        assert!(resp.success, "Second split should succeed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 50.0);

        // 支付剩余 (Item A 的 1 个 + Item C): 20 + 10 = 30
        pay_order(&manager, &order_id, 30.0, "CASH");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 80.0);

        // 完成
        let complete_resp = complete_order(&manager, &order_id);
        assert!(complete_resp.success);
        assert_order_status(&manager, &order_id, OrderStatus::Completed);
    }

    // ------------------------------------------------------------------------
    // P0.5: 多次金额分单后完成
    // ------------------------------------------------------------------------
    #[test]
    fn test_split_by_amount_multiple_then_complete() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-amount-split",
            vec![simple_item("product:p1", "Total Item", 100.0, 1)],
        );

        // 第一次金额分单: 30%
        let split_cmd1 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByAmount {
                order_id: order_id.clone(),
                split_amount: 30.0,
                payment_method: "CASH".to_string(),
                tendered: Some(30.0),
            },
        );
        let resp = manager.execute_command(split_cmd1);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 30.0);
        assert!(snapshot.has_amount_split, "has_amount_split should be true");

        // 第二次金额分单: 30%
        let split_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByAmount {
                order_id: order_id.clone(),
                split_amount: 30.0,
                payment_method: "CARD".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(split_cmd2);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 60.0);

        // 支付剩余 40%
        pay_order(&manager, &order_id, 40.0, "CASH");

        // 完成
        let complete_resp = complete_order(&manager, &order_id);
        assert!(complete_resp.success);
        assert_order_status(&manager, &order_id, OrderStatus::Completed);
    }

    // ------------------------------------------------------------------------
    // P0.6: AA 分单 3 人不能整除场景 (精度测试)
    // ------------------------------------------------------------------------
    #[test]
    fn test_aa_split_three_payers_indivisible() {
        let manager = create_test_manager();

        // 100 元订单，3 人 AA
        let order_id = open_table_with_items(
            &manager,
            "T-aa-3way",
            vec![simple_item("product:p1", "Shared Meal", 100.0, 1)],
        );

        // StartAaSplit: 3 人，先付 1 份
        let start_aa = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::StartAaSplit {
                order_id: order_id.clone(),
                total_shares: 3,
                shares: 1,
                payment_method: "CASH".to_string(),
                tendered: Some(34.0), // 给 34 块
            },
        );
        let resp = manager.execute_command(start_aa);
        assert!(resp.success, "StartAaSplit should succeed");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.aa_total_shares, Some(3));
        assert_eq!(snapshot.aa_paid_shares, 1);
        // 100 / 3 ≈ 33.33，实际支付第一份
        let first_share = snapshot.paid_amount;
        assert!(first_share > 33.0 && first_share < 34.0, "First share should be ~33.33");

        // PayAaSplit: 第二份
        let pay_aa_2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::PayAaSplit {
                order_id: order_id.clone(),
                shares: 1,
                payment_method: "CARD".to_string(),
                tendered: None,
            },
        );
        let resp = manager.execute_command(pay_aa_2);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.aa_paid_shares, 2);

        // PayAaSplit: 第三份 (最后一份应该拿剩余金额)
        let pay_aa_3 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::PayAaSplit {
                order_id: order_id.clone(),
                shares: 1,
                payment_method: "CASH".to_string(),
                tendered: Some(34.0),
            },
        );
        let resp = manager.execute_command(pay_aa_3);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.aa_paid_shares, 3);
        // 精度验证: 总支付应该恰好等于 100.0
        let diff = (snapshot.paid_amount - 100.0).abs();
        assert!(diff < 0.01, "Total paid should be exactly 100.0, got {}", snapshot.paid_amount);

        // 完成
        let complete_resp = complete_order(&manager, &order_id);
        assert!(complete_resp.success);
    }

    // ------------------------------------------------------------------------
    // P0.7: 合并订单后修改并完成
    // ------------------------------------------------------------------------
    #[test]
    fn test_merge_orders_then_modify_then_complete() {
        let manager = create_test_manager();

        // 源订单
        let source_id = open_table_with_items(
            &manager,
            "T-merge-src-2",
            vec![simple_item("product:p1", "Coffee", 5.0, 2)], // 10
        );

        // 目标订单
        let target_id = open_table_with_items(
            &manager,
            "T-merge-tgt-2",
            vec![simple_item("product:p2", "Tea", 4.0, 1)], // 4
        );

        // 合并
        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: source_id.clone(),
                target_order_id: target_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(resp.success, "Merge should succeed");

        // 验证源订单状态
        assert_order_status(&manager, &source_id, OrderStatus::Merged);

        // 验证目标订单
        let target = manager.get_snapshot(&target_id).unwrap().unwrap();
        assert_eq!(target.items.len(), 2, "Target should have 2 items after merge");
        assert_eq!(target.total, 14.0); // 10 + 4

        // 继续在目标订单添加商品
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: target_id.clone(),
                items: vec![simple_item("product:p3", "Cake", 6.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success, "AddItems to merged target should succeed");

        let target = manager.get_snapshot(&target_id).unwrap().unwrap();
        assert_eq!(target.items.len(), 3);
        assert_eq!(target.total, 20.0);

        // 支付并完成
        pay_order(&manager, &target_id, 20.0, "CARD");
        let complete_resp = complete_order(&manager, &target_id);
        assert!(complete_resp.success);
    }

    // ------------------------------------------------------------------------
    // P0.8: 移桌后合并再完成
    // ------------------------------------------------------------------------
    #[test]
    fn test_move_then_merge_then_complete() {
        let manager = create_test_manager();

        // 订单 1: T1
        let order1 = open_table_with_items(
            &manager,
            "T1-move",
            vec![simple_item("product:p1", "Item 1", 10.0, 1)],
        );

        // 移桌: T1 → T2
        let move_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order1.clone(),
                target_table_id: "T2-move".to_string(),
                target_table_name: "Table 2".to_string(),
                target_zone_id: Some("zone:z2".to_string()),
                target_zone_name: Some("Zone B".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order1).unwrap().unwrap();
        assert_eq!(snapshot.table_id, Some("T2-move".to_string()));
        assert_eq!(snapshot.zone_id, Some("zone:z2".to_string()));

        // 订单 2: T3
        let order2 = open_table_with_items(
            &manager,
            "T3-move",
            vec![simple_item("product:p2", "Item 2", 20.0, 1)],
        );

        // 合并: T2 (order1) → T3 (order2)
        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: order1.clone(),
                target_order_id: order2.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(merge_cmd);
        assert!(resp.success);

        // order1 应该是 Merged 状态
        assert_order_status(&manager, &order1, OrderStatus::Merged);

        // order2 应该有所有商品
        let target = manager.get_snapshot(&order2).unwrap().unwrap();
        assert_eq!(target.items.len(), 2);
        assert_eq!(target.total, 30.0);

        // 完成
        pay_order(&manager, &order2, 30.0, "CASH");
        let complete_resp = complete_order(&manager, &order2);
        assert!(complete_resp.success);
    }

    // ------------------------------------------------------------------------
    // P0.9: 商品添加→修改→移除链条 (金额重算验证)
    // ------------------------------------------------------------------------
    #[test]
    fn test_items_add_modify_remove_chain() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-chain",
            vec![simple_item("product:p1", "Test Item", 10.0, 5)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();
        assert_eq!(snapshot.subtotal, 50.0);

        // ModifyItem: 数量 5 → 3
        let modify_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                affected_quantity: None,
                changes: shared::order::ItemChanges {
                    quantity: Some(3),
                    ..Default::default()
                },
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(modify_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items[0].quantity, 3);
        assert_eq!(snapshot.subtotal, 30.0);

        // RemoveItem: 移除 2 个
        let remove_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::RemoveItem {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: Some(2),
                reason: Some("Customer changed mind".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(remove_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items[0].quantity, 1);
        assert_eq!(snapshot.subtotal, 10.0);

        // 再移除剩下的 1 个
        let remove_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::RemoveItem {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: None, // 移除全部
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(remove_cmd2);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.items.is_empty() || snapshot.items.iter().all(|i| i.quantity == 0));
        assert_eq!(snapshot.subtotal, 0.0);
    }

    // ========================================================================
    // ========================================================================
    //  P1: 金额计算准确性测试
    // ========================================================================
    // ========================================================================

    // ------------------------------------------------------------------------
    // P1.1: 部分移除后金额重算
    // ------------------------------------------------------------------------
    #[test]
    fn test_remove_item_partial_recalculates() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-partial-remove",
            vec![simple_item("product:p1", "Item", 10.0, 5)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 移除 2 个
        let remove_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::RemoveItem {
                order_id: order_id.clone(),
                instance_id,
                quantity: Some(2),
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(remove_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items[0].quantity, 3);
        assert_eq!(snapshot.subtotal, 30.0);
        assert_eq!(snapshot.total, 30.0);
    }

    // ------------------------------------------------------------------------
    // P1.2: 折扣 + 附加费叠加
    // ------------------------------------------------------------------------
    #[test]
    fn test_discount_plus_surcharge_calculation() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-disc-sur",
            vec![simple_item("product:p1", "Item", 100.0, 1)],
        );

        // 应用 10% 折扣
        let discount_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: Some(10.0),
                discount_fixed: None,
                reason: Some("VIP".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(discount_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.order_manual_discount_percent, Some(10.0));
        assert_eq!(snapshot.order_manual_discount_amount, 10.0);

        // 应用 15 元附加费
        let surcharge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: None,
                surcharge_amount: Some(15.0),
                reason: Some("Service fee".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(surcharge_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.order_manual_surcharge_fixed, Some(15.0));
        // total = 100 - 10 + 15 = 105
        assert_eq!(snapshot.total, 105.0);
    }

    // ------------------------------------------------------------------------
    // P1.3: 商品级折扣 + 订单级折扣叠加
    // ------------------------------------------------------------------------
    #[test]
    fn test_item_level_plus_order_level_discount() {
        let manager = create_test_manager();

        // 添加带 10% 手动折扣的商品
        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-dual-disc".to_string()),
                table_name: Some("Table".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![item_with_discount("product:p1", "Item", 100.0, 1, 10.0)], // 90 after item discount
            },
        );
        manager.execute_command(add_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 商品级折扣后 subtotal = 90
        assert_eq!(snapshot.subtotal, 90.0);

        // 应用 5% 订单级折扣
        let discount_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: Some(5.0),
                discount_fixed: None,
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(discount_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // total = 90 - (90 * 5%) = 90 - 4.5 = 85.5
        assert_eq!(snapshot.total, 85.5);
    }

    // ------------------------------------------------------------------------
    // P1.4: Comp 后 Uncomp 恢复价格
    // ------------------------------------------------------------------------
    #[test]
    fn test_comp_then_uncomp_restores_price() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-comp-uncomp",
            vec![simple_item("product:p1", "Item", 25.0, 2)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();
        assert_eq!(snapshot.total, 50.0);

        // Comp 2 个
        let comp_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: 2,
                reason: "Birthday gift".to_string(),
                authorizer_id: "auth:1".to_string(),
                authorizer_name: "Manager".to_string(),
            },
        );
        let resp = manager.execute_command(comp_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // Comp 后，原商品应该变成 comped，价格为 0
        let comped_item = snapshot.items.iter().find(|i| i.is_comped).unwrap();
        assert_eq!(comped_item.quantity, 2);
        assert_eq!(snapshot.total, 0.0);
        assert_eq!(snapshot.comp_total_amount, 50.0);

        // Uncomp
        let uncomp_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::UncompItem {
                order_id: order_id.clone(),
                instance_id: comped_item.instance_id.clone(),
                authorizer_id: "auth:1".to_string(),
                authorizer_name: "Manager".to_string(),
            },
        );
        let resp = manager.execute_command(uncomp_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // Uncomp 后价格恢复
        assert_eq!(snapshot.total, 50.0);
        assert_eq!(snapshot.comp_total_amount, 0.0);
    }

    // ------------------------------------------------------------------------
    // P1.5: 部分 Comp 创建拆分商品
    // ------------------------------------------------------------------------
    #[test]
    fn test_comp_partial_creates_split_item() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-partial-comp",
            vec![simple_item("product:p1", "Item", 10.0, 5)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // Comp 2 个 (部分)
        let comp_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: 2,
                reason: "Promotion".to_string(),
                authorizer_id: "auth:1".to_string(),
                authorizer_name: "Manager".to_string(),
            },
        );
        let resp = manager.execute_command(comp_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 应该有 2 个商品项: 原始 3 个 + comped 2 个
        assert_eq!(snapshot.items.len(), 2);

        let normal_item = snapshot.items.iter().find(|i| !i.is_comped).unwrap();
        let comped_item = snapshot.items.iter().find(|i| i.is_comped).unwrap();

        assert_eq!(normal_item.quantity, 3);
        assert_eq!(comped_item.quantity, 2);
        assert!(comped_item.instance_id.contains("::comp::"));

        // total = 3 * 10 = 30 (comped 部分不计)
        assert_eq!(snapshot.total, 30.0);
        assert_eq!(snapshot.comp_total_amount, 20.0);
    }

    // ------------------------------------------------------------------------
    // P1.6: 大金额精度测试
    // ------------------------------------------------------------------------
    #[test]
    fn test_large_order_precision() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-large-precision",
            vec![simple_item("product:p1", "Expensive", 99999.99, 100)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 99999.99 * 100 = 9999999.0
        assert_eq!(snapshot.subtotal, 9999999.0);
        assert_eq!(snapshot.total, 9999999.0);

        // 验证无精度丢失
        let expected = 99999.99 * 100.0;
        let diff = (snapshot.total - expected).abs();
        assert!(diff < 0.01, "Precision loss detected: expected {}, got {}", expected, snapshot.total);
    }

    // ------------------------------------------------------------------------
    // P1.7: 选项价格修改器累加
    // ------------------------------------------------------------------------
    #[test]
    fn test_option_price_modifiers_accumulate() {
        let manager = create_test_manager();

        let open_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T-options".to_string()),
                table_name: Some("Table".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![item_with_options(
                    "product:pizza",
                    "Pizza",
                    15.0,
                    1,
                    vec![
                        shared::order::ItemOption {
                            attribute_id: "attr:size".to_string(),
                            attribute_name: "Size".to_string(),
                            option_idx: 1,
                            option_name: "Large".to_string(),
                            price_modifier: Some(5.0),
                            quantity: 1,
                        },
                        shared::order::ItemOption {
                            attribute_id: "attr:topping".to_string(),
                            attribute_name: "Topping".to_string(),
                            option_idx: 0,
                            option_name: "Extra Cheese".to_string(),
                            price_modifier: Some(2.5),
                            quantity: 1,
                        },
                    ],
                )],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 15 + 5 + 2.5 = 22.5
        assert_eq!(snapshot.subtotal, 22.5);
    }

    // ========================================================================
    // ========================================================================
    //  P2: 状态转换边界测试
    // ========================================================================
    // ========================================================================

    // ------------------------------------------------------------------------
    // P2.1: 已完成订单拒绝所有修改命令
    // ------------------------------------------------------------------------
    #[test]
    fn test_all_commands_reject_completed_order() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-completed-reject",
            vec![simple_item("product:p1", "Item", 10.0, 1)],
        );

        // 支付并完成
        pay_order(&manager, &order_id, 10.0, "CASH");
        complete_order(&manager, &order_id);
        assert_order_status(&manager, &order_id, OrderStatus::Completed);

        // 测试 AddItems
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p2", "New Item", 5.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "AddItems should fail on completed order");

        // 测试 AddPayment
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 5.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "AddPayment should fail on completed order");

        // 测试 VoidOrder
        let void_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::Cancelled,
                loss_reason: None,
                loss_amount: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(void_cmd);
        assert!(!resp.success, "VoidOrder should fail on completed order");

        // 测试 MoveOrder
        let move_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: "T-new".to_string(),
                target_table_name: "New Table".to_string(),
                target_zone_id: None,
                target_zone_name: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(!resp.success, "MoveOrder should fail on completed order");
    }

    // ------------------------------------------------------------------------
    // P2.2: 已作废订单拒绝所有修改命令
    // ------------------------------------------------------------------------
    #[test]
    fn test_all_commands_reject_voided_order() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-voided-reject",
            vec![simple_item("product:p1", "Item", 10.0, 1)],
        );

        // 作废订单
        void_order_helper(&manager, &order_id, VoidType::Cancelled);
        assert_order_status(&manager, &order_id, OrderStatus::Void);

        // 测试 AddItems
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item("product:p2", "New Item", 5.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "AddItems should fail on voided order");

        // 测试 AddPayment
        let pay_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: 5.0,
                    tendered: None,
                    note: None,
                },
            },
        );
        let resp = manager.execute_command(pay_cmd);
        assert!(!resp.success, "AddPayment should fail on voided order");

        // 测试 CompleteOrder
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(!resp.success, "CompleteOrder should fail on voided order");
    }

    // ------------------------------------------------------------------------
    // P2.3: 已合并订单 - 验证合并后源订单状态
    // 注意: 当前实现对 Merged 状态订单不检查 AddItems，但订单已不在活跃列表
    // ------------------------------------------------------------------------
    #[test]
    fn test_merged_order_not_in_active_list() {
        let manager = create_test_manager();

        let source_id = open_table_with_items(
            &manager,
            "T-merged-source",
            vec![simple_item("product:p1", "Item", 10.0, 1)],
        );
        let target_id = open_table_with_items(
            &manager,
            "T-merged-target",
            vec![simple_item("product:p2", "Item 2", 10.0, 1)],
        );

        // 合并前两个订单都在活跃列表
        let active = manager.get_active_orders().unwrap();
        assert_eq!(active.len(), 2);

        // 合并
        let merge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::MergeOrders {
                source_order_id: source_id.clone(),
                target_order_id: target_id.clone(),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(merge_cmd);
        assert_order_status(&manager, &source_id, OrderStatus::Merged);

        // 合并后源订单不在活跃列表
        let active = manager.get_active_orders().unwrap();
        assert_eq!(active.len(), 1);
        assert!(active.iter().all(|o| o.order_id != source_id), "Merged order should not be in active list");
        assert!(active.iter().any(|o| o.order_id == target_id), "Target order should be in active list");
    }

    // ------------------------------------------------------------------------
    // P2.4: UpdateOrderInfo 不影响金额
    // ------------------------------------------------------------------------
    #[test]
    fn test_update_guest_count_does_not_affect_totals() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-update-info",
            vec![simple_item("product:p1", "Item", 100.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.guest_count, 2); // 默认值
        assert_eq!(snapshot.total, 100.0);

        // 更新 guest_count
        let update_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::UpdateOrderInfo {
                order_id: order_id.clone(),
                guest_count: Some(8),
                table_name: None,
                is_pre_payment: None,
            },
        );
        let resp = manager.execute_command(update_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.guest_count, 8);
        assert_eq!(snapshot.total, 100.0, "Total should not change after updating guest_count");
    }

    // ------------------------------------------------------------------------
    // P2.5: AddOrderNote 覆盖之前的备注
    // ------------------------------------------------------------------------
    #[test]
    fn test_add_note_overwrites_previous() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(&manager, "T-note", vec![]);

        // 添加第一个备注
        let note_cmd1 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddOrderNote {
                order_id: order_id.clone(),
                note: "First note".to_string(),
            },
        );
        manager.execute_command(note_cmd1);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.note, Some("First note".to_string()));

        // 添加第二个备注 (应覆盖)
        let note_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddOrderNote {
                order_id: order_id.clone(),
                note: "Second note".to_string(),
            },
        );
        manager.execute_command(note_cmd2);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.note, Some("Second note".to_string()));
    }

    // ------------------------------------------------------------------------
    // P2.6: 空字符串清除备注
    // ------------------------------------------------------------------------
    #[test]
    fn test_clear_note_with_empty_string() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(&manager, "T-clear-note", vec![]);

        // 添加备注
        let note_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddOrderNote {
                order_id: order_id.clone(),
                note: "Some note".to_string(),
            },
        );
        manager.execute_command(note_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.note, Some("Some note".to_string()));

        // 用空字符串清除
        let clear_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::AddOrderNote {
                order_id: order_id.clone(),
                note: String::new(),
            },
        );
        manager.execute_command(clear_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.note.is_none() || snapshot.note.as_deref() == Some(""));
    }

    // ========================================================================
    // ========================================================================
    //  P3: 边界条件与错误处理测试
    // ========================================================================
    // ========================================================================

    // ------------------------------------------------------------------------
    // P3.1: 支付→取消→支付循环
    // ------------------------------------------------------------------------
    #[test]
    fn test_add_cancel_add_payment_cycle() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-pay-cycle",
            vec![simple_item("product:p1", "Item", 30.0, 1)],
        );

        // 第一次支付
        pay_order(&manager, &order_id, 30.0, "CARD");
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let payment1_id = snapshot.payments[0].payment_id.clone();
        assert_eq!(snapshot.paid_amount, 30.0);

        // 取消第一次支付
        let cancel_cmd1 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CancelPayment {
                order_id: order_id.clone(),
                payment_id: payment1_id,
                reason: Some("Wrong card".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(cancel_cmd1);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 0.0);

        // 第二次支付
        pay_order(&manager, &order_id, 30.0, "CASH");
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let payment2_id = snapshot.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
        assert_eq!(snapshot.paid_amount, 30.0);

        // 取消第二次支付
        let cancel_cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CancelPayment {
                order_id: order_id.clone(),
                payment_id: payment2_id,
                reason: Some("Customer changed mind".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(cancel_cmd2);
        assert!(resp.success);

        // 第三次支付
        pay_order(&manager, &order_id, 30.0, "CARD");

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.payments.len(), 3);
        assert_eq!(
            snapshot.payments.iter().filter(|p| p.cancelled).count(),
            2,
            "Should have 2 cancelled payments"
        );
        assert_eq!(snapshot.paid_amount, 30.0);

        // 完成订单
        let complete_resp = complete_order(&manager, &order_id);
        assert!(complete_resp.success);
    }

    // ------------------------------------------------------------------------
    // P3.2: AA 分单不能与菜品分单混用
    // ------------------------------------------------------------------------
    #[test]
    fn test_aa_split_cannot_mix_with_item_split() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-aa-no-mix",
            vec![simple_item("product:p1", "Item", 100.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 开始 AA 分单
        let start_aa = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::StartAaSplit {
                order_id: order_id.clone(),
                total_shares: 2,
                shares: 1,
                payment_method: "CASH".to_string(),
                tendered: Some(100.0),
            },
        );
        let resp = manager.execute_command(start_aa);
        assert!(resp.success);

        // 尝试菜品分单应该失败
        let item_split_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByItems {
                order_id: order_id.clone(),
                payment_method: "CARD".to_string(),
                items: vec![shared::order::SplitItem {
                    instance_id,
                    name: "Item".to_string(),
                    quantity: 1,
                    unit_price: 100.0,
                }],
                tendered: None,
            },
        );
        let resp = manager.execute_command(item_split_cmd);
        assert!(!resp.success, "Item split should fail when AA split is active");
    }

    // ------------------------------------------------------------------------
    // P3.3: Comp 后支付再完成
    // ------------------------------------------------------------------------
    #[test]
    fn test_comp_then_pay_then_complete() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-comp-pay",
            vec![
                simple_item("product:p1", "Item A", 10.0, 1),
                simple_item("product:p2", "Item B", 10.0, 1),
            ],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item_a_id = snapshot.items.iter().find(|i| i.name == "Item A").unwrap().instance_id.clone();
        assert_eq!(snapshot.total, 20.0);

        // Comp Item A
        let comp_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id: item_a_id,
                quantity: 1,
                reason: "Gift".to_string(),
                authorizer_id: "auth:1".to_string(),
                authorizer_name: "Manager".to_string(),
            },
        );
        let resp = manager.execute_command(comp_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // total 应该只包含 Item B: 10.0
        assert_eq!(snapshot.total, 10.0);

        // 支付 10.0
        pay_order(&manager, &order_id, 10.0, "CASH");

        // 完成
        let complete_resp = complete_order(&manager, &order_id);
        assert!(complete_resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
    }

    // ------------------------------------------------------------------------
    // P3.4: 金额分单后菜品分单被禁用
    // ------------------------------------------------------------------------
    #[test]
    fn test_amount_split_blocks_item_split() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-amount-blocks-item",
            vec![simple_item("product:p1", "Item", 100.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 金额分单
        let amount_split = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByAmount {
                order_id: order_id.clone(),
                split_amount: 50.0,
                payment_method: "CASH".to_string(),
                tendered: Some(50.0),
            },
        );
        let resp = manager.execute_command(amount_split);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.has_amount_split);

        // 尝试菜品分单应该失败
        let item_split = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByItems {
                order_id: order_id.clone(),
                payment_method: "CARD".to_string(),
                items: vec![shared::order::SplitItem {
                    instance_id,
                    name: "Item".to_string(),
                    quantity: 1,
                    unit_price: 100.0,
                }],
                tendered: None,
            },
        );
        let resp = manager.execute_command(item_split);
        assert!(!resp.success, "Item split should be blocked when amount split is active");
    }

    // ------------------------------------------------------------------------
    // P3.6: 取消不存在的支付应失败
    // ------------------------------------------------------------------------
    #[test]
    fn test_cancel_nonexistent_payment_fails() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-cancel-nonexistent",
            vec![simple_item("product:p1", "Item", 10.0, 1)],
        );

        let cancel_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CancelPayment {
                order_id: order_id.clone(),
                payment_id: "nonexistent-payment-id".to_string(),
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(cancel_cmd);
        assert!(!resp.success, "CancelPayment should fail for nonexistent payment");
    }

    // ------------------------------------------------------------------------
    // P3.7: 超额菜品分单应失败
    // ------------------------------------------------------------------------
    #[test]
    fn test_split_by_items_overpay_fails() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-split-overpay",
            vec![simple_item("product:p1", "Item", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 尝试支付超过可用数量
        let split_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByItems {
                order_id: order_id.clone(),
                payment_method: "CASH".to_string(),
                items: vec![shared::order::SplitItem {
                    instance_id,
                    name: "Item".to_string(),
                    quantity: 5, // 订单只有 1 个
                    unit_price: 10.0,
                }],
                tendered: Some(50.0),
            },
        );
        let resp = manager.execute_command(split_cmd);
        assert!(!resp.success, "Split with excessive quantity should fail");
    }

    // ------------------------------------------------------------------------
    // P3.8: 移除超过现有数量应失败
    // ------------------------------------------------------------------------
    #[test]
    fn test_remove_item_excessive_quantity_fails() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-remove-excess",
            vec![simple_item("product:p1", "Item", 10.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 尝试移除 5 个，但只有 2 个
        let remove_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::RemoveItem {
                order_id: order_id.clone(),
                instance_id,
                quantity: Some(5),
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(remove_cmd);
        assert!(!resp.success, "Remove with excessive quantity should fail");
    }

    // ------------------------------------------------------------------------
    // P3.9: Comp 超过现有数量应失败
    // ------------------------------------------------------------------------
    #[test]
    fn test_comp_item_excessive_quantity_fails() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-comp-excess",
            vec![simple_item("product:p1", "Item", 10.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 尝试 comp 5 个，但只有 2 个
        let comp_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id,
                quantity: 5,
                reason: "Test".to_string(),
                authorizer_id: "auth:1".to_string(),
                authorizer_name: "Manager".to_string(),
            },
        );
        let resp = manager.execute_command(comp_cmd);
        assert!(!resp.success, "Comp with excessive quantity should fail");
    }

    // ------------------------------------------------------------------------
    // P3.10: 清除整单折扣
    // ------------------------------------------------------------------------
    #[test]
    fn test_clear_order_discount() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-clear-discount",
            vec![simple_item("product:p1", "Item", 100.0, 1)],
        );

        // 应用折扣
        let discount_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: Some(20.0),
                discount_fixed: None,
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(discount_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 80.0);

        // 清除折扣 (两个参数都为 None)
        let clear_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: None,
                discount_fixed: None,
                reason: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(clear_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 100.0);
        assert!(snapshot.order_manual_discount_percent.is_none());
    }

    // ------------------------------------------------------------------------
    // P3.11: ToggleRuleSkip - 规则不存在时应失败
    // 注意: ToggleRuleSkip 需要订单中有 applied_rules，否则会失败
    // ------------------------------------------------------------------------
    #[test]
    fn test_toggle_rule_skip_nonexistent_rule_fails() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-rule-skip",
            vec![simple_item("product:p1", "Item", 100.0, 1)],
        );

        // 尝试 toggle 不存在的规则应失败
        let toggle_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ToggleRuleSkip {
                order_id: order_id.clone(),
                rule_id: "nonexistent-rule".to_string(),
                skipped: true,
            },
        );
        let resp = manager.execute_command(toggle_cmd);
        assert!(!resp.success, "ToggleRuleSkip should fail when rule not found");
    }

    // ------------------------------------------------------------------------
    // P3.12: 固定金额折扣
    // ------------------------------------------------------------------------
    #[test]
    fn test_fixed_amount_discount() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-fixed-discount",
            vec![simple_item("product:p1", "Item", 100.0, 1)],
        );

        // 应用 25 元固定折扣
        let discount_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: None,
                discount_fixed: Some(25.0),
                reason: Some("Coupon".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(discount_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.order_manual_discount_fixed, Some(25.0));
        assert_eq!(snapshot.total, 75.0);
    }

    // ------------------------------------------------------------------------
    // P3.13: 百分比附加费
    // ------------------------------------------------------------------------
    #[test]
    fn test_percentage_surcharge() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            "T-pct-surcharge",
            vec![simple_item("product:p1", "Item", 100.0, 1)],
        );

        // 应用 10% 附加费
        let surcharge_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: Some(10.0),
                surcharge_amount: None,
                reason: Some("Service charge".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(surcharge_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.order_manual_surcharge_percent, Some(10.0));
        assert_eq!(snapshot.total, 110.0);
    }
}
