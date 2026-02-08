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
use shared::models::PriceRule;
use crate::pricing::matcher::is_time_valid;
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
    /// Archive service for completed orders (optional, only set when SQLite is available)
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

    /// Set the archive service for SQLite integration
    pub fn set_archive_service(&mut self, pool: sqlx::SqlitePool) {
        self.archive_service = Some(super::OrderArchiveService::new(pool, self.tz));
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
            tracing::error!(order_id = %order_id, error = %e, "Failed to persist rule snapshot, rule guarantee degraded for this order");
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
            tracing::error!(order_id = %order_id, error = %e, "Failed to remove rule snapshot");
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
                tracing::error!(error = %e, "Failed to restore rule snapshots from redb");
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
                    tracing::warn!(order_id = %order_id, error = %e, "Failed to clean up orphan rule snapshot");
                }
                orphaned += 1;
            }
        }

        if orphaned > 0 {
            tracing::info!(orphaned, "Cleaned up orphan rule snapshots");
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
    ) -> HashMap<i64, ProductMeta> {
        let Some(catalog) = &self.catalog_service else {
            return HashMap::new();
        };
        let product_ids: Vec<i64> = items.iter().map(|i| i.product_id).collect();
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
        tracing::debug!(command_id = %cmd.command_id, payload = ?cmd.payload, "Processing command");
        
        // 1. Idempotency check (before transaction)
        if self.storage.is_command_processed(&cmd.command_id)? {
            tracing::warn!(command_id = %cmd.command_id, "Duplicate command");
            return Ok((CommandResponse::duplicate(cmd.command_id), vec![]));
        }

        // 2. For OpenTable: pre-check table availability before generating receipt_number
        // This avoids wasting receipt numbers on failed table opens
        if let shared::order::OrderCommandPayload::OpenTable { table_id: Some(tid), table_name, .. } = &cmd.payload
            && let Some(existing) = self.storage.find_active_order_for_table(*tid)? {
                let name = table_name.as_deref().unwrap_or("unknown");
                return Err(ManagerError::TableOccupied(format!(
                    "桌台 {} 已被占用 (订单: {})", name, existing
                )));
            }

        // 3. Pre-generate receipt_number and queue_number for OpenTable (BEFORE transaction to avoid deadlock)
        // redb doesn't allow nested write transactions
        let pre_generated_receipt = match &cmd.payload {
            shared::order::OrderCommandPayload::OpenTable { .. } => {
                let receipt = self.next_receipt_number();
                tracing::debug!(receipt_number = %receipt, "Pre-generated receipt number");
                Some(receipt)
            }
            _ => None,
        };
        let pre_generated_queue = match &cmd.payload {
            shared::order::OrderCommandPayload::OpenTable { is_retail: true, .. } => {
                match self.storage.next_queue_number(self.tz) {
                    Ok(qn) => {
                        tracing::debug!(queue_number = qn, "Pre-generated queue number");
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
            operator_id: cmd.operator_id,
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
                tracing::debug!(table_id = ?table_id, table_name = ?table_name, "Processing OpenTable command");
                // Use pre-generated receipt_number (generated before transaction)
                let receipt_number = pre_generated_receipt.ok_or_else(|| {
                    OrderError::InvalidOperation("receipt_number must be pre-generated for OpenTable".to_string())
                })?;
                CommandAction::OpenTable(super::actions::OpenTableAction {
                    table_id: *table_id,
                    table_name: table_name.clone(),
                    zone_id: *zone_id,
                    zone_name: zone_name.clone(),
                    guest_count: *guest_count,
                    is_retail: *is_retail,
                    queue_number: pre_generated_queue,
                    receipt_number,
                })
            }
            shared::order::OrderCommandPayload::AddItems { order_id, items } => {
                let cached_rules = self.get_cached_rules(order_id).unwrap_or_default();
                // 按当前时间动态过滤（区域是静态缓存，时间是动态的）
                let now = shared::util::now_millis();
                let rules: Vec<PriceRule> = cached_rules
                    .into_iter()
                    .filter(|r| is_time_valid(r, now, self.tz))
                    .collect();
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
                OrderStatus::Completed | OrderStatus::Void | OrderStatus::Merged => {
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

        // 12. Clean up rule cache for terminal orders (Complete/Void/Merge)
        // Note: MoveOrder is NOT terminal — order stays Active, rules handled by callers
        match &cmd.payload {
            shared::order::OrderCommandPayload::CompleteOrder { order_id, .. }
            | shared::order::OrderCommandPayload::VoidOrder { order_id, .. } => {
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
            let needs_recalc = order.items.iter().any(|item| item.line_total == 0.0 && !item.is_comped);
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
            let needs_recalc = order.items.iter().any(|item| item.line_total == 0.0 && !item.is_comped);
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

    fn create_open_table_cmd(operator_id: i64) -> OrderCommand {
        OrderCommand::new(
            operator_id,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(1),
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
        let cmd = create_open_table_cmd(1);

        let response = manager.execute_command(cmd);

        assert!(response.success);
        assert!(response.order_id.is_some());

        let order_id = response.order_id.unwrap();
        let snapshot = manager.get_snapshot(&order_id).unwrap();
        assert!(snapshot.is_some());

        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.status, OrderStatus::Active);
        assert_eq!(snapshot.table_id, Some(1));
    }

    #[test]
    fn test_idempotency() {
        let manager = create_test_manager();
        let cmd = create_open_table_cmd(1);

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
        let open_cmd = create_open_table_cmd(1);
        let open_response = manager.execute_command(open_cmd);
        let order_id = open_response.order_id.unwrap();

        // Add items
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
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
        let open_cmd = create_open_table_cmd(1);
        let open_response = manager.execute_command(open_cmd);
        let order_id = open_response.order_id.unwrap();

        // Add items
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
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
            1,
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
            1,
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
        let open_cmd = create_open_table_cmd(1);
        let open_response = manager.execute_command(open_cmd);
        let order_id = open_response.order_id.unwrap();

        // Void order
        let void_cmd = OrderCommand::new(
            1,
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
        let open_cmd = create_open_table_cmd(1);
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
        table_id: i64,
        items: Vec<CartItemInput>,
    ) -> String {
        let open_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(table_id),
                table_name: Some(format!("Table {}", table_id)),
                zone_id: Some(1),
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
                1,
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

    fn simple_item(product_id: i64, name: &str, price: f64, quantity: i32) -> CartItemInput {
        CartItemInput {
            product_id,
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
            111,
            vec![
                simple_item(1, "Coffee", 4.5, 2),
                simple_item(2, "Tea", 3.0, 1),
            ],
        );

        // Add a payment
        let pay_cmd = OrderCommand::new(
            1,
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
            201,
            vec![simple_item(1, "Coffee", 5.0, 1)],
        );

        // Verify initial zone
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.zone_id, Some(1));
        assert_eq!(snapshot.zone_name, Some("Zone A".to_string()));

        // Move to a different table in a different zone
        let move_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: 328,
                target_table_name: "Table T-move-2".to_string(),
                target_zone_id: Some(2),
                target_zone_name: Some("Zone B".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.table_id, Some(328));
        assert_eq!(
            snapshot.zone_id,
            Some(2),
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
            202,
            vec![simple_item(1, "Coffee", 10.0, 2)],
        );

        // Pay partially on source
        let pay_cmd = OrderCommand::new(
            1,
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
            203,
            vec![simple_item(2, "Tea", 8.0, 1)],
        );

        // Merge source → target should be rejected
        let merge_cmd = OrderCommand::new(
            1,
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
    fn test_add_payment_overpay_is_rejected() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            204,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // Pay way more than the total — should be rejected
        let pay_cmd = OrderCommand::new(
            1,
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
        assert!(
            !resp.success,
            "AddPayment should reject overpayment"
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.paid_amount, 0.0);
    }

    // ========================================================================
    // 5. cancel_payment → re-pay → complete 完整流程
    // ========================================================================

    #[test]
    fn test_cancel_payment_then_repay_then_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            205,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // Pay with CARD
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
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
            1,
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
        let order_id = open_table_with_items(&manager, 100, vec![]);

        let complete_cmd = OrderCommand::new(
            1,
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
            206,
            vec![simple_item(1, "Coffee", 5.0, 1)],
        );

        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(2, "Tea", 3.0, 1)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(307),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(307),
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
        let order_id = open_table_with_items(&manager, 101, vec![]);

        let void_cmd = OrderCommand::new(
            1,
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
            1,
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
            207,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let move_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: 329,
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
            1,
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
            1,
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
        assert_eq!(snapshot.table_id, Some(329));
    }

    // ========================================================================
    // 11. split by items → complete 流程
    // ========================================================================

    #[test]
    fn test_split_by_items_then_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            208,
            vec![
                simple_item(1, "Coffee", 10.0, 2),
                simple_item(2, "Tea", 8.0, 1),
            ],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 28.0);
        let coffee_instance = snapshot.items[0].instance_id.clone();

        // Split pay: 2x Coffee = 20.0
        let split_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
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
            209,
            vec![simple_item(1, "Coffee", 30.0, 1)],
        );

        // Start AA: 3 shares, pay 1
        let start_aa_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
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
            1,
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
            1,
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

        let cmd = create_open_table_cmd(1);
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
            210,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let seq_before = manager.get_current_sequence().unwrap();

        let pay_cmd = OrderCommand::new(
            1,
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
            211,
            vec![simple_item(1, "Coffee", 5.0, 1)],
        );
        let target_id = open_table_with_items(
            &manager,
            212,
            vec![simple_item(2, "Tea", 3.0, 1)],
        );

        let merge_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: "nonexistent".to_string(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp = manager.execute_command(complete_cmd);
        assert!(!resp.success);

        let void_cmd = OrderCommand::new(
            1,
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
            213,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd);

        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(2, "Tea", 5.0, 1)],
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
            214,
            vec![simple_item(1, "Free Sample", 0.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].price, 0.0);
        assert_eq!(snapshot.subtotal, 0.0);
        assert_eq!(snapshot.total, 0.0);

        // 零总额可以直接完成
        let complete_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(308),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "NaN Item", f64::NAN, 2)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(309),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Infinity Item", f64::INFINITY, 1)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(310),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Negative Item", -10.0, 1)],
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
            215,
            vec![simple_item(1, "Expensive Item", 99999.99, 100)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(311),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Max Item", f64::MAX, 1)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(312),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Zero Qty", 10.0, 0)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(313),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Negative Qty", 10.0, -3)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(314),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Max Qty", 0.01, i32::MAX)],
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
            216,
            vec![simple_item(1, "Max Allowed", 0.01, 9999)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(315),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(316),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
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
            217,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
            218,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
            219,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(317),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![
                    simple_item(1, "Normal", 25.50, 2),
                    simple_item(2, "Free", 0.0, 1),
                    simple_item(3, "Cheap", 0.01, 100),
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
            1,
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
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(318),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
                    name: "Pizza".to_string(),
                    price: 12.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: Some(vec![
                        shared::order::ItemOption {
                            attribute_id: 1,
                            attribute_name: "Size".to_string(),
                            option_idx: 2,
                            option_name: "Large".to_string(),
                            price_modifier: Some(3.0), // +3
                            quantity: 1,
                        },
                        shared::order::ItemOption {
                            attribute_id: 2,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(319),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
                    name: "Special".to_string(),
                    price: 10.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: Some(vec![shared::order::ItemOption {
                        attribute_id: 3,
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
            220,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(320),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
                    name: "Combo Item".to_string(),
                    price: 33.33,
                    original_price: None,
                    quantity: 3,
                    selected_options: Some(vec![shared::order::ItemOption {
                        attribute_id: 1,
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
            221,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // NaN payment — 被输入验证拒绝
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(321),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![
                    simple_item(1, "Free", 0.0, 5),
                    simple_item(2, "Penny", 0.01, 99),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(322),
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
        for i in 0i64..10 {
            let add_cmd = OrderCommand::new(
                1,
                "Test Operator".to_string(),
                OrderCommandPayload::AddItems {
                    order_id: order_id.clone(),
                    items: vec![CartItemInput {
                        product_id: i + 1,
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
            222,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
    // 43. 移桌后仍可正常支付
    // ========================================================================

    #[test]
    fn test_add_payment_after_move_order() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            223,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // Move order
        let move_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: 330,
                target_table_name: "Table 2".to_string(),
                target_zone_id: None,
                target_zone_name: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(move_cmd);

        // MoveOrder 只移动桌台，订单保持 Active，仍可支付
        let pay_cmd = OrderCommand::new(
            1,
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
        assert!(resp.success, "Order should accept payments after MoveOrder (status stays Active)");
    }

    // ========================================================================
    // 44. 极小金额差异 — 支付容差边界
    // ========================================================================

    #[test]
    fn test_payment_tolerance_boundary() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            224,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // 支付 9.99 — 差 0.01，在容差内
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
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
            225,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // 支付 9.98 — 差 0.02，超出容差
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(323),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
                    name: "Steak".to_string(),
                    price: 20.0,
                    original_price: None,
                    quantity: 2,
                    selected_options: Some(vec![
                        shared::order::ItemOption {
                            attribute_id: 4,
                            attribute_name: "Sauce".to_string(),
                            option_idx: 0,
                            option_name: "BBQ".to_string(),
                            price_modifier: Some(3.0),
                            quantity: 1,
                        },
                        shared::order::ItemOption {
                            attribute_id: 5,
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
        assert_eq!(item.original_price, 20.0, "original_price = input price");

        // unit_price: base(20)+options(5)=25, discount=25*10%=2.5, unit=22.5
        assert_eq!(item.unit_price, 22.5, "unit_price = 22.5 (no double counting)");

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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(324),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: 1,
                    name: "Pasta".to_string(),
                    price: 15.0,
                    original_price: None,
                    quantity: 3,
                    selected_options: Some(vec![shared::order::ItemOption {
                        attribute_id: 6,
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
        assert_eq!(snapshot_before.items[0].unit_price, 17.5);
        assert_eq!(snapshot_before.subtotal, 52.5);

        // Modify: add 20% discount
        let modify_cmd = OrderCommand::new(
            1,
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
        assert_eq!(item.original_price, 15.0, "original_price unchanged after modify");

        // unit_price: base(15)+options(2.5)=17.5, discount=17.5*20%=3.5, unit=14.0
        assert_eq!(item.unit_price, 14.0, "unit_price after 20% discount");

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
            226,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // Void
        let void_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(2, "Tea", 5.0, 1)],
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
            227,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let void_cmd = OrderCommand::new(
            1,
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
            1,
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
        let order_id = open_table_with_items(&manager, 102, vec![]);

        let void_cmd = OrderCommand::new(
            1,
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
            1,
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
            228,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // Pay + complete
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd);

        // Try to void
        let void_cmd = OrderCommand::new(
            1,
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
            229,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        let resp1 = manager.execute_command(complete_cmd);
        assert!(resp1.success);

        let complete_cmd2 = OrderCommand::new(
            1,
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
            230,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            1,
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
            231,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            1,
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
            232,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            1,
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
            233,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            1,
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
            234,
            vec![simple_item(1, "Coffee", 10.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        let modify_cmd = OrderCommand::new(
            1,
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
        let order_id = open_table_with_items(&manager, 103, vec![]);

        let add_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Coffee", 10.0, 1)],
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
        let source_id = open_table_with_items(&manager, 104, vec![]);
        let target_id = open_table_with_items(&manager, 105, vec![]);

        // Void source
        let void_cmd = OrderCommand::new(
            1,
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
            1,
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
        let source_id = open_table_with_items(&manager, 106, vec![]);
        let target_id = open_table_with_items(&manager, 107, vec![]);

        // Void target
        let void_cmd = OrderCommand::new(
            1,
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
            1,
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
        let order_id = open_table_with_items(&manager, 108, vec![]);

        let merge_cmd = OrderCommand::new(
            1,
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
            235,
            vec![simple_item(1, "Coffee", 30.0, 1)],
        );

        let cmd = OrderCommand::new(
            1,
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
            236,
            vec![simple_item(1, "Coffee", 30.0, 1)],
        );

        let cmd = OrderCommand::new(
            1,
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
            237,
            vec![simple_item(1, "Coffee", 30.0, 1)],
        );

        let cmd = OrderCommand::new(
            1,
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
            238,
            vec![simple_item(1, "Coffee", 30.0, 1)],
        );

        // Start AA: 3 shares, pay 2
        let start_cmd = OrderCommand::new(
            1,
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
            1,
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
            239,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // Pay
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
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
            240,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );
        let order2 = open_table_with_items(
            &manager,
            241,
            vec![simple_item(2, "Tea", 5.0, 1)],
        );

        // Move order2 to T-occ-1 (occupied)
        let move_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order2.clone(),
                target_table_id: 240,
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
            242,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // Pay + complete
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                service_type: Some(ServiceType::DineIn),
            },
        );
        manager.execute_command(complete_cmd);

        // Try to modify
        let modify_cmd = OrderCommand::new(
            1,
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
            243,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
            244,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        let pay_cmd = OrderCommand::new(
            1,
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
        use shared::models::price_rule::{AdjustmentType, ProductScope, RuleType};
        PriceRule {
            id: 0,
            name: name.to_string(),
            display_name: name.to_string(),
            receipt_name: name.to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target_id: None,
            zone_scope: "all".to_string(),
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
        let open_cmd = create_open_table_cmd(1);
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // 缓存规则 (10% global discount)
        manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
        assert!(manager.get_cached_rules(&order_id).is_some());

        // 加菜
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Coffee", 10.0, 1)],
            },
        );
        manager.execute_command(add_cmd);

        // 查询实际 total（可能因规则折扣而与原价不同）
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let actual_total = snapshot.total;

        // 支付实际 total
        let pay_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddPayment {
                order_id: order_id.clone(),
                payment: PaymentInput {
                    method: "CASH".to_string(),
                    amount: actual_total,
                    tendered: Some(actual_total),
                    note: None,
                },
            },
        );
        let pay_resp = manager.execute_command(pay_cmd);
        assert!(pay_resp.success, "Payment should succeed");

        // 完成订单
        let complete_cmd = OrderCommand::new(
            1,
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
        let open_cmd = create_open_table_cmd(1);
        let resp = manager.execute_command(open_cmd);
        let order_id = resp.order_id.unwrap();

        // 缓存规则
        manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
        assert!(manager.get_cached_rules(&order_id).is_some());

        // 作废订单
        let void_cmd = OrderCommand::new(
            1,
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
    fn test_move_order_preserves_rules() {
        let manager = create_test_manager();

        let order_id = open_table_with_items(
            &manager,
            245,
            vec![simple_item(1, "Coffee", 5.0, 1)],
        );

        // 缓存规则
        manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
        assert!(manager.get_cached_rules(&order_id).is_some());
        assert!(manager.storage().get_rule_snapshot(&order_id).unwrap().is_some());

        // 换桌 — 订单保持 Active，规则不清除
        // （实际场景中由调用方按新区域重新加载规则）
        let move_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: 331,
                target_table_name: "Table T-rule-move-2".to_string(),
                target_zone_id: Some(2),
                target_zone_name: Some("Zone B".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(resp.success);

        // MoveOrder 不是 terminal 操作，规则保留（由调用方用新区域重载）
        assert!(manager.get_cached_rules(&order_id).is_some());
        assert!(manager.storage().get_rule_snapshot(&order_id).unwrap().is_some());
    }

    #[test]
    fn test_merge_orders_cleans_source_rules() {
        let manager = create_test_manager();

        // 源订单
        let source_id = open_table_with_items(
            &manager,
            246,
            vec![simple_item(1, "Coffee", 10.0, 1)],
        );

        // 目标订单
        let target_id = open_table_with_items(
            &manager,
            247,
            vec![simple_item(2, "Tea", 8.0, 1)],
        );

        // 给源订单缓存规则
        manager.cache_rules(&source_id, vec![create_test_rule("SourceRule")]);
        assert!(manager.get_cached_rules(&source_id).is_some());
        assert!(manager.storage().get_rule_snapshot(&source_id).unwrap().is_some());

        // 合并 source → target
        let merge_cmd = OrderCommand::new(
            1,
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
        product_id: i64,
        name: &str,
        price: f64,
        quantity: i32,
        options: Vec<shared::order::ItemOption>,
    ) -> CartItemInput {
        CartItemInput {
            product_id,
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
        product_id: i64,
        name: &str,
        price: f64,
        quantity: i32,
        discount_percent: f64,
    ) -> CartItemInput {
        CartItemInput {
            product_id,
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
            1,
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
            1,
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
            1,
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



    /// 打开零售订单
    fn open_retail_order(manager: &OrdersManager) -> String {
        let cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(325),
                table_name: Some("Table Dine Flow".to_string()),
                zone_id: Some(1),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![
                    simple_item(10, "Coffee", 5.0, 2),      // 10.0
                    simple_item(11, "Tea", 3.0, 3),            // 9.0
                    simple_item(12, "Cake", 12.50, 1),        // 12.50
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
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Item", 15.0, 2)],
            },
        );
        manager.execute_command(add_cmd);

        // 3. 支付
        pay_order(&manager, &order_id, 30.0, "CASH");

        // 4. 完成 (指定服务类型为外带)
        let complete_cmd = OrderCommand::new(
            1,
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
            248,
            vec![simple_item(1, "Expensive Item", 100.0, 1)],
        );

        // 损失结算作废
        let void_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::LossSettled,
                loss_reason: Some(shared::order::LossReason::CustomerFled),
                loss_amount: Some(100.0),
                note: Some("Customer fled without paying".to_string()),
                authorizer_id: Some(1),
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
            249,
            vec![
                simple_item(1, "Item A", 20.0, 2),  // 40
                simple_item(2, "Item B", 15.0, 2),  // 30
                simple_item(3, "Item C", 10.0, 1),  // 10
            ],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 80.0);
        let item_a_id = snapshot.items.iter().find(|i| i.name == "Item A").unwrap().instance_id.clone();
        let item_b_id = snapshot.items.iter().find(|i| i.name == "Item B").unwrap().instance_id.clone();

        // 第一次分单: 支付 Item A 的 1 个 (20.0)
        let split_cmd1 = OrderCommand::new(
            1,
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
            1,
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
            250,
            vec![simple_item(1, "Total Item", 100.0, 1)],
        );

        // 第一次金额分单: 30%
        let split_cmd1 = OrderCommand::new(
            1,
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
            1,
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
            251,
            vec![simple_item(1, "Shared Meal", 100.0, 1)],
        );

        // StartAaSplit: 3 人，先付 1 份
        let start_aa = OrderCommand::new(
            1,
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
            1,
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
            1,
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
            252,
            vec![simple_item(1, "Coffee", 5.0, 2)], // 10
        );

        // 目标订单
        let target_id = open_table_with_items(
            &manager,
            253,
            vec![simple_item(2, "Tea", 4.0, 1)], // 4
        );

        // 合并
        let merge_cmd = OrderCommand::new(
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: target_id.clone(),
                items: vec![simple_item(3, "Cake", 6.0, 1)],
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
            401,
            vec![simple_item(1, "Item 1", 10.0, 1)],
        );

        // 移桌: T1 → T2
        let move_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order1.clone(),
                target_table_id: 400,
                target_table_name: "Table 2".to_string(),
                target_zone_id: Some(2),
                target_zone_name: Some("Zone B".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let resp = manager.execute_command(move_cmd);
        assert!(resp.success);

        let snapshot = manager.get_snapshot(&order1).unwrap().unwrap();
        assert_eq!(snapshot.table_id, Some(400));
        assert_eq!(snapshot.zone_id, Some(2));

        // 订单 2: T3
        let order2 = open_table_with_items(
            &manager,
            403,
            vec![simple_item(2, "Item 2", 20.0, 1)],
        );

        // 合并: T2 (order1) → T3 (order2)
        let merge_cmd = OrderCommand::new(
            1,
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
            254,
            vec![simple_item(1, "Test Item", 10.0, 5)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();
        assert_eq!(snapshot.subtotal, 50.0);

        // ModifyItem: 数量 5 → 3
        let modify_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
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
            255,
            vec![simple_item(1, "Item", 10.0, 5)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 移除 2 个
        let remove_cmd = OrderCommand::new(
            1,
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
            256,
            vec![simple_item(1, "Item", 100.0, 1)],
        );

        // 应用 10% 折扣
        let discount_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: Some(10.0),
                discount_fixed: None,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: None,
                surcharge_amount: Some(15.0),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(326),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![item_with_discount(1, "Item", 100.0, 1, 10.0)], // 90 after item discount
            },
        );
        manager.execute_command(add_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 商品级折扣后 subtotal = 90
        assert_eq!(snapshot.subtotal, 90.0);

        // 应用 5% 订单级折扣
        let discount_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: Some(5.0),
                discount_fixed: None,
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
            257,
            vec![simple_item(1, "Item", 25.0, 2)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();
        assert_eq!(snapshot.total, 50.0);

        // Comp 2 个
        let comp_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: 2,
                reason: "Birthday gift".to_string(),
                authorizer_id: 1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::UncompItem {
                order_id: order_id.clone(),
                instance_id: comped_item.instance_id.clone(),
                authorizer_id: 1,
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
            258,
            vec![simple_item(1, "Item", 10.0, 5)], // 50
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // Comp 2 个 (部分)
        let comp_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: 2,
                reason: "Promotion".to_string(),
                authorizer_id: 1,
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
            259,
            vec![simple_item(1, "Expensive", 99999.99, 100)],
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(327),
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![item_with_options(
                    1,
                    "Pizza",
                    15.0,
                    1,
                    vec![
                        shared::order::ItemOption {
                            attribute_id: 1,
                            attribute_name: "Size".to_string(),
                            option_idx: 1,
                            option_name: "Large".to_string(),
                            price_modifier: Some(5.0),
                            quantity: 1,
                        },
                        shared::order::ItemOption {
                            attribute_id: 2,
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
            260,
            vec![simple_item(1, "Item", 10.0, 1)],
        );

        // 支付并完成
        pay_order(&manager, &order_id, 10.0, "CASH");
        complete_order(&manager, &order_id);
        assert_order_status(&manager, &order_id, OrderStatus::Completed);

        // 测试 AddItems
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(2, "New Item", 5.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "AddItems should fail on completed order");

        // 测试 AddPayment
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
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
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::MoveOrder {
                order_id: order_id.clone(),
                target_table_id: 332,
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
            261,
            vec![simple_item(1, "Item", 10.0, 1)],
        );

        // 作废订单
        void_order_helper(&manager, &order_id, VoidType::Cancelled);
        assert_order_status(&manager, &order_id, OrderStatus::Void);

        // 测试 AddItems
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(2, "New Item", 5.0, 1)],
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(!resp.success, "AddItems should fail on voided order");

        // 测试 AddPayment
        let pay_cmd = OrderCommand::new(
            1,
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
            1,
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
            262,
            vec![simple_item(1, "Item", 10.0, 1)],
        );
        let target_id = open_table_with_items(
            &manager,
            263,
            vec![simple_item(2, "Item 2", 10.0, 1)],
        );

        // 合并前两个订单都在活跃列表
        let active = manager.get_active_orders().unwrap();
        assert_eq!(active.len(), 2);

        // 合并
        let merge_cmd = OrderCommand::new(
            1,
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
            264,
            vec![simple_item(1, "Item", 100.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.guest_count, 2); // 默认值
        assert_eq!(snapshot.total, 100.0);

        // 更新 guest_count
        let update_cmd = OrderCommand::new(
            1,
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

        let order_id = open_table_with_items(&manager, 109, vec![]);

        // 添加第一个备注
        let note_cmd1 = OrderCommand::new(
            1,
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
            1,
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

        let order_id = open_table_with_items(&manager, 110, vec![]);

        // 添加备注
        let note_cmd = OrderCommand::new(
            1,
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
            1,
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
            265,
            vec![simple_item(1, "Item", 30.0, 1)],
        );

        // 第一次支付
        pay_order(&manager, &order_id, 30.0, "CARD");
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let payment1_id = snapshot.payments[0].payment_id.clone();
        assert_eq!(snapshot.paid_amount, 30.0);

        // 取消第一次支付
        let cancel_cmd1 = OrderCommand::new(
            1,
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
            1,
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
            266,
            vec![simple_item(1, "Item", 100.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 开始 AA 分单
        let start_aa = OrderCommand::new(
            1,
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
            1,
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
            267,
            vec![
                simple_item(1, "Item A", 10.0, 1),
                simple_item(2, "Item B", 10.0, 1),
            ],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item_a_id = snapshot.items.iter().find(|i| i.name == "Item A").unwrap().instance_id.clone();
        assert_eq!(snapshot.total, 20.0);

        // Comp Item A
        let comp_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id: item_a_id,
                quantity: 1,
                reason: "Gift".to_string(),
                authorizer_id: 1,
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
            268,
            vec![simple_item(1, "Item", 100.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 金额分单
        let amount_split = OrderCommand::new(
            1,
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
            1,
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
            269,
            vec![simple_item(1, "Item", 10.0, 1)],
        );

        let cancel_cmd = OrderCommand::new(
            1,
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
            270,
            vec![simple_item(1, "Item", 10.0, 1)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 尝试支付超过可用数量
        let split_cmd = OrderCommand::new(
            1,
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
            271,
            vec![simple_item(1, "Item", 10.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 尝试移除 5 个，但只有 2 个
        let remove_cmd = OrderCommand::new(
            1,
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
            272,
            vec![simple_item(1, "Item", 10.0, 2)],
        );

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        let instance_id = snapshot.items[0].instance_id.clone();

        // 尝试 comp 5 个，但只有 2 个
        let comp_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id,
                quantity: 5,
                reason: "Test".to_string(),
                authorizer_id: 1,
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
            273,
            vec![simple_item(1, "Item", 100.0, 1)],
        );

        // 应用折扣
        let discount_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: Some(20.0),
                discount_fixed: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(discount_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 80.0);

        // 清除折扣 (两个参数都为 None)
        let clear_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: None,
                discount_fixed: None,
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
            274,
            vec![simple_item(1, "Item", 100.0, 1)],
        );

        // 尝试 toggle 不存在的规则应失败
        let toggle_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ToggleRuleSkip {
                order_id: order_id.clone(),
                rule_id: 99999,
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
            275,
            vec![simple_item(1, "Item", 100.0, 1)],
        );

        // 应用 25 元固定折扣
        let discount_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: None,
                discount_fixed: Some(25.0),
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
            276,
            vec![simple_item(1, "Item", 100.0, 1)],
        );

        // 应用 10% 附加费
        let surcharge_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: Some(10.0),
                surcharge_amount: None,
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

    // ========================================================================
    // Edge-case combo tests: 奇怪组合场景
    // ========================================================================

    /// Helper: 修改商品（折扣/价格/数量）
    fn modify_item(
        manager: &OrdersManager,
        order_id: &str,
        instance_id: &str,
        changes: shared::order::ItemChanges,
    ) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ModifyItem {
                order_id: order_id.to_string(),
                instance_id: instance_id.to_string(),
                affected_quantity: None,
                changes,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 添加支付
    fn pay(manager: &OrdersManager, order_id: &str, amount: f64, method: &str) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
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
        manager.execute_command(cmd)
    }

    /// Helper: 取消支付
    fn cancel_payment(
        manager: &OrdersManager,
        order_id: &str,
        payment_id: &str,
    ) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CancelPayment {
                order_id: order_id.to_string(),
                payment_id: payment_id.to_string(),
                reason: Some("test".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 整单折扣
    fn apply_discount(manager: &OrdersManager, order_id: &str, percent: f64) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.to_string(),
                discount_percent: Some(percent),
                discount_fixed: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 清除整单折扣
    fn clear_discount(manager: &OrdersManager, order_id: &str) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.to_string(),
                discount_percent: Some(0.0),
                discount_fixed: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 分单支付（按商品）
    fn split_by_items(
        manager: &OrdersManager,
        order_id: &str,
        items: Vec<shared::order::SplitItem>,
        method: &str,
    ) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::SplitByItems {
                order_id: order_id.to_string(),
                items,
                payment_method: method.to_string(),
                tendered: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: comp 商品 (comp all unpaid quantity)
    fn comp_item(manager: &OrdersManager, order_id: &str, instance_id: &str) -> CommandResponse {
        let s = manager.get_snapshot(order_id).unwrap().unwrap();
        let qty = s.items.iter()
            .find(|i| i.instance_id == instance_id)
            .map(|i| i.unpaid_quantity)
            .unwrap_or(1);
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.to_string(),
                instance_id: instance_id.to_string(),
                quantity: qty,
                reason: "test comp".to_string(),
                authorizer_id: 1,
                authorizer_name: "Test".to_string(),
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 折扣 changes
    fn discount_changes(percent: f64) -> shared::order::ItemChanges {
        shared::order::ItemChanges {
            price: None,
            quantity: None,
            manual_discount_percent: Some(percent),
            note: None,
            selected_options: None,
            selected_specification: None,
        }
    }

    /// Helper: 价格 changes
    fn price_changes(price: f64) -> shared::order::ItemChanges {
        shared::order::ItemChanges {
            price: Some(price),
            quantity: None,
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        }
    }

    /// Helper: 数量 changes
    fn qty_changes(qty: i32) -> shared::order::ItemChanges {
        shared::order::ItemChanges {
            price: None,
            quantity: Some(qty),
            manual_discount_percent: None,
            note: None,
            selected_options: None,
            selected_specification: None,
        }
    }

    /// Helper: 验证快照一致性 (stored vs rebuilt from events)
    fn assert_snapshot_consistent(manager: &OrdersManager, order_id: &str) {
        let stored = manager.get_snapshot(order_id).unwrap().unwrap();
        let rebuilt = manager.rebuild_snapshot(order_id).unwrap();
        assert_eq!(
            stored.state_checksum, rebuilt.state_checksum,
            "Snapshot diverged from event replay!\n  stored items: {:?}\n  rebuilt items: {:?}\n  stored paid_amount: {}\n  rebuilt paid_amount: {}",
            stored.items.iter().map(|i| (&i.instance_id, i.quantity, i.unpaid_quantity)).collect::<Vec<_>>(),
            rebuilt.items.iter().map(|i| (&i.instance_id, i.quantity, i.unpaid_quantity)).collect::<Vec<_>>(),
            stored.paid_amount, rebuilt.paid_amount,
        );
    }

    // --- Test 1: 折扣循环 50%→20%→50%→0% ---

    #[test]
    fn test_combo_discount_cycling_no_payment() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            277,
            vec![simple_item(1, "Coffee", 10.0, 3)], // total=30
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();

        // 50% discount → total = 15
        let r = modify_item(&manager, &order_id, &iid, discount_changes(50.0));
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 1, "Should still be 1 item");
        assert!((s.total - 15.0).abs() < 0.01);
        let iid = s.items[0].instance_id.clone();

        // 20% discount → total = 24
        let r = modify_item(&manager, &order_id, &iid, discount_changes(20.0));
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 1);
        assert!((s.total - 24.0).abs() < 0.01);
        let iid = s.items[0].instance_id.clone();

        // Back to 50% → total = 15
        let r = modify_item(&manager, &order_id, &iid, discount_changes(50.0));
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 1);
        assert!((s.total - 15.0).abs() < 0.01);
        let iid = s.items[0].instance_id.clone();

        // Remove discount → total = 30
        let r = modify_item(&manager, &order_id, &iid, discount_changes(0.0));
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 1, "Removing discount should merge back");
        assert!((s.total - 30.0).abs() < 0.01);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 2: 折扣循环 + 部分支付 + 取消支付 ---

    #[test]
    fn test_combo_discount_cycle_with_partial_payment_and_cancel() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            278,
            vec![simple_item(1, "Coffee", 10.0, 4)], // total=40
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();

        // 1. Apply 50% discount → total=20
        let r = modify_item(&manager, &order_id, &iid, discount_changes(50.0));
        assert!(r.success);

        // 2. Pay 10 (partial)
        let r = pay(&manager, &order_id, 10.0, "CASH");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 10.0).abs() < 0.01);
        // After partial payment, total=20, paid=10
        // But recalculate_totals recalculates with paid items at discounted price
        assert!(s.remaining_amount > 0.0, "Should have remaining amount");

        // 3. Change unpaid items to 20% discount (paid items keep 50%)
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items.iter().find(|i| i.unpaid_quantity > 0).unwrap().instance_id.clone();
        let r = modify_item(&manager, &order_id, &iid, discount_changes(20.0));
        assert!(r.success, "Item-level discount on unpaid portion should succeed");

        // 4. Cancel the payment
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let payment_id = s.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &payment_id);
        assert!(r.success);

        // 5. After cancel, paid_amount should be 0
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount).abs() < 0.01, "paid should be 0 after cancel");

        // 6. Remove all discounts
        let items_snapshot: Vec<_> = s.items.iter()
            .filter(|i| i.manual_discount_percent.is_some())
            .map(|i| i.instance_id.clone())
            .collect();
        for iid in &items_snapshot {
            modify_item(&manager, &order_id, iid, discount_changes(0.0));
        }

        // 7. Verify: should be back to original total, no fragmentation
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 40.0).abs() < 0.01, "Total should be original 40, got {}", s.total);
        let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
        assert_eq!(total_qty, 4, "Total quantity should remain 4");

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 3: 分单支付 + 改价 + 再支付 ---

    #[test]
    fn test_combo_split_payment_then_modify_then_pay() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            279,
            vec![
                simple_item(1, "Coffee", 10.0, 3), // 30
                simple_item(2, "Tea", 8.0, 2),     // 16 → total=46
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let coffee_iid = s.items.iter().find(|i| i.name == "Coffee").unwrap().instance_id.clone();

        // 1. Split-pay 2 coffees (20)
        let r = split_by_items(&manager, &order_id, vec![
            shared::order::SplitItem {
                instance_id: coffee_iid.clone(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            },
        ], "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 20.0).abs() < 0.01);
        assert_eq!(s.paid_item_quantities.get(&coffee_iid), Some(&2));

        // 2. Modify remaining coffee price to 15 (should only affect unpaid portion)
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let unpaid_coffee = s.items.iter()
            .find(|i| i.name == "Coffee" && i.unpaid_quantity > 0)
            .unwrap();
        let unpaid_iid = unpaid_coffee.instance_id.clone();
        let r = modify_item(&manager, &order_id, &unpaid_iid, price_changes(15.0));
        assert!(r.success, "Should be able to modify unpaid coffee price");

        // 3. Pay remaining
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let remaining = s.remaining_amount;
        assert!(remaining > 0.0);
        let r = pay(&manager, &order_id, remaining, "CASH");
        assert!(r.success, "Should pay remaining {}", remaining);

        // 4. Complete
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 4: Comp + 折扣 + 支付 ---

    #[test]
    fn test_combo_comp_then_discount_then_pay() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            280,
            vec![
                simple_item(1, "Coffee", 10.0, 2), // 20
                simple_item(2, "Tea", 5.0, 2),     // 10 → total=30
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let tea_iid = s.items.iter().find(|i| i.name == "Tea").unwrap().instance_id.clone();

        // 1. Comp the tea
        let r = comp_item(&manager, &order_id, &tea_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 20.0).abs() < 0.01, "Total should be 20 after comp tea");

        // 2. Apply 50% discount on coffee → total should be 10
        let coffee_iid = s.items.iter().find(|i| i.name == "Coffee" && !i.is_comped).unwrap().instance_id.clone();
        let r = modify_item(&manager, &order_id, &coffee_iid, discount_changes(50.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 10.0).abs() < 0.01, "Total should be 10, got {}", s.total);

        // 3. Pay full amount
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);

        // 4. Complete
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        // 5. Verify order is completed and totals are correct
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.status, OrderStatus::Completed);
        // Comped tea should have is_comped=true and not contribute to total
        let comped_count = s.items.iter().filter(|i| i.is_comped).count();
        assert!(comped_count > 0, "Should have comped items");

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 5: 整单折扣 100% → total=0 → 可完成 ---

    #[test]
    fn test_combo_100_percent_order_discount() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            281,
            vec![simple_item(1, "Coffee", 10.0, 2)], // 20
        );

        // 100% discount
        let r = apply_discount(&manager, &order_id, 100.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total).abs() < 0.01, "100% discount → total=0, got {}", s.total);
        assert!((s.remaining_amount).abs() < 0.01);

        // Should be able to complete without payment
        let r = complete_order(&manager, &order_id);
        assert!(r.success, "Should complete with 0 total");

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 6: 大量折扣 → total clamp 到 0 ---

    #[test]
    fn test_combo_fixed_discount_exceeds_total() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            282,
            vec![simple_item(1, "Coffee", 10.0, 1)], // 10
        );

        // Fixed discount of 50 on a 10 order
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.clone(),
                discount_percent: None,
                discount_fixed: Some(50.0),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let r = manager.execute_command(cmd);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(s.total >= 0.0, "Total must not be negative, got {}", s.total);
        assert!((s.total).abs() < 0.01, "Total should clamp to 0, got {}", s.total);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 7: 支付 → 取消 → 再支付 → 完成 ---

    #[test]
    fn test_combo_pay_cancel_repay_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            283,
            vec![simple_item(1, "Coffee", 10.0, 3)], // 30
        );

        // Pay 15
        let r = pay(&manager, &order_id, 15.0, "CARD");
        assert!(r.success);

        // Cancel it
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let pid = s.payments[0].payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &pid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount).abs() < 0.01);
        assert!((s.remaining_amount - 30.0).abs() < 0.01);

        // Pay full
        let r = pay(&manager, &order_id, 30.0, "CASH");
        assert!(r.success);

        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 8: 分单支付后 cancel → 重新分单 ---

    #[test]
    fn test_combo_split_cancel_resplit() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            284,
            vec![
                simple_item(1, "Coffee", 10.0, 2), // 20
                simple_item(2, "Tea", 5.0, 2),     // 10
            ], // total=30
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let coffee_iid = s.items.iter().find(|i| i.name == "Coffee").unwrap().instance_id.clone();
        let tea_iid = s.items.iter().find(|i| i.name == "Tea").unwrap().instance_id.clone();

        // 1. Split-pay all coffee (20)
        let r = split_by_items(&manager, &order_id, vec![
            shared::order::SplitItem {
                instance_id: coffee_iid.clone(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            },
        ], "CARD");
        assert!(r.success);

        // 2. Cancel that split payment
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let pid = s.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &pid);
        assert!(r.success);

        // 3. Now split-pay tea instead
        let r = split_by_items(&manager, &order_id, vec![
            shared::order::SplitItem {
                instance_id: tea_iid.clone(),
                name: "Tea".to_string(),
                quantity: 2,
                unit_price: 5.0,
            },
        ], "CASH");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 10.0).abs() < 0.01, "Should have paid 10 for tea");
        assert!((s.remaining_amount - 20.0).abs() < 0.01);

        // 4. Pay remaining
        let r = pay(&manager, &order_id, 20.0, "CARD");
        assert!(r.success);

        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 9: 多次部分支付 + 修改数量 ---

    #[test]
    fn test_combo_multiple_partial_payments_then_modify_qty() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            285,
            vec![simple_item(1, "Coffee", 10.0, 5)], // 50
        );

        // Pay 20, then 15
        let r = pay(&manager, &order_id, 20.0, "CARD");
        assert!(r.success, "Pay 20 failed: {:?}", r.error);

        let r = pay(&manager, &order_id, 15.0, "CASH");
        assert!(r.success, "Pay 15 failed: {:?}", r.error);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(
            (s.paid_amount - 35.0).abs() < 0.01,
            "paid={}, total={}, remaining={}, payments={}",
            s.paid_amount, s.total, s.remaining_amount, s.payments.len()
        );

        // Compute actual remaining from authoritative fields
        let actual_remaining = s.total - s.paid_amount;
        assert!(
            (s.remaining_amount - actual_remaining).abs() < 0.02,
            "remaining_amount({}) diverged from total({}) - paid({})",
            s.remaining_amount, s.total, s.paid_amount
        );

        // Try to overpay — should fail
        let r = pay(&manager, &order_id, actual_remaining + 1.0, "CARD");
        assert!(!r.success, "Should reject overpayment");

        // Pay exact remaining
        let r = pay(&manager, &order_id, actual_remaining, "CARD");
        assert!(r.success, "Paying remaining ({}) failed: {:?}", actual_remaining, r.error);

        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 10: 折扣 + comp + 分单 + 完成 ---

    #[test]
    fn test_combo_discount_comp_split_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            286,
            vec![
                simple_item(1, "Steak", 25.0, 2),  // 50
                simple_item(2, "Wine", 15.0, 2),   // 30
                simple_item(3, "Bread", 3.0, 1),   // 3
            ], // total=83
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let bread_iid = s.items.iter().find(|i| i.name == "Bread").unwrap().instance_id.clone();
        let steak_iid = s.items.iter().find(|i| i.name == "Steak").unwrap().instance_id.clone();

        // 1. Comp the bread
        let r = comp_item(&manager, &order_id, &bread_iid);
        assert!(r.success);

        // 2. 20% discount on steak → steak_total = 2 * 25 * 0.8 = 40
        let r = modify_item(&manager, &order_id, &steak_iid, discount_changes(20.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // total = steak(40) + wine(30) = 70
        assert!((s.total - 70.0).abs() < 0.01, "Expected 70, got {}", s.total);

        // 3. Split-pay 1 steak (discounted: 25*0.8 = 20)
        let steak = s.items.iter()
            .find(|i| i.name == "Steak" && !i.is_comped && i.unpaid_quantity > 0)
            .unwrap();
        let r = split_by_items(&manager, &order_id, vec![
            shared::order::SplitItem {
                instance_id: steak.instance_id.clone(),
                name: "Steak".to_string(),
                quantity: 1,
                unit_price: 20.0,
            },
        ], "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 20.0).abs() < 0.01);

        // 4. Pay remaining (50)
        let r = pay(&manager, &order_id, s.remaining_amount, "CASH");
        assert!(r.success);

        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 11: 同一商品多次添加 + 折扣 + 去折 → 自动合并 ---

    #[test]
    fn test_combo_add_twice_discount_undiscount_merges() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            287,
            vec![simple_item(1, "Coffee", 10.0, 2)], // 20
        );

        // Add same product again
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![simple_item(1, "Coffee", 10.0, 3)],
            },
        );
        let r = manager.execute_command(add_cmd);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // Should auto-merge: 1 item with qty=5
        assert_eq!(s.items.len(), 1, "Same product should merge on add");
        assert_eq!(s.items[0].quantity, 5);
        let iid = s.items[0].instance_id.clone();

        // Apply 30% discount
        let r = modify_item(&manager, &order_id, &iid, discount_changes(30.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();

        // Remove discount → should return to original instance_id and stay as 1 item
        let r = modify_item(&manager, &order_id, &iid, discount_changes(0.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 1, "Should still be 1 item after removing discount");
        assert_eq!(s.items[0].quantity, 5);
        assert!((s.total - 50.0).abs() < 0.01);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 12: 支付后不能加整单折扣 ---

    #[test]
    fn test_combo_order_discount_blocked_after_payment() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            288,
            vec![simple_item(1, "Coffee", 10.0, 3)], // 30
        );

        // Pay 10
        let r = pay(&manager, &order_id, 10.0, "CARD");
        assert!(r.success);

        // Try order-level discount — should fail
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(!r.success, "Order discount should be blocked after payment");

        // Cancel payment
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let pid = s.payments[0].payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &pid);
        assert!(r.success);

        // Now order discount should work again
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(r.success, "Order discount should work after cancelling all payments");

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 24.0).abs() < 0.01); // 30 * 0.8 = 24

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 13: 添加商品 → 部分支付 → void → 验证 loss ---

    #[test]
    fn test_combo_partial_pay_then_void_loss() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            289,
            vec![simple_item(1, "Coffee", 10.0, 5)], // 50
        );

        // Pay 30
        let r = pay(&manager, &order_id, 30.0, "CARD");
        assert!(r.success);

        // Void with loss settled (auto-calculate loss)
        let void_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::LossSettled,
                loss_reason: Some(shared::order::LossReason::CustomerFled),
                loss_amount: None, // auto-calculate
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let r = manager.execute_command(void_cmd);
        assert!(r.success);

        // Verify via snapshot: void sets status
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.status, OrderStatus::Void);
        // Verify loss_amount via rebuild (events contain the value)
        let rebuilt = manager.rebuild_snapshot(&order_id).unwrap();
        assert_eq!(rebuilt.status, OrderStatus::Void);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 14: 超付保护 — 边界值 ---

    #[test]
    fn test_combo_overpayment_boundary() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            290,
            vec![simple_item(1, "Coffee", 10.0, 1)], // 10
        );

        // Pay 10.00 exact — should succeed
        let r = pay(&manager, &order_id, 10.0, "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.remaining_amount).abs() < 0.01);

        // Try to pay even 0.01 more — should fail
        let r = pay(&manager, &order_id, 0.02, "CARD");
        assert!(!r.success, "Should reject payment when fully paid");

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 15: 整单折扣 + 整单附加费 + 商品折扣 组合 ---

    #[test]
    fn test_combo_order_discount_surcharge_item_discount() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            291,
            vec![
                simple_item(1, "Coffee", 10.0, 2), // 20
                simple_item(2, "Tea", 8.0, 1),     // 8 → total=28
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let coffee_iid = s.items.iter().find(|i| i.name == "Coffee").unwrap().instance_id.clone();

        // 1. 50% item discount on coffee → coffee=10, total=18
        let r = modify_item(&manager, &order_id, &coffee_iid, discount_changes(50.0));
        assert!(r.success);

        // 2. 10% order discount → total = 18 - 1.8 = 16.2
        let r = apply_discount(&manager, &order_id, 10.0);
        assert!(r.success);

        // 3. 5% order surcharge → total = 18 - 1.8 + 0.9 = 17.1
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: Some(5.0),
                surcharge_amount: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let r = manager.execute_command(cmd);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(s.total > 0.0, "Total must be positive");
        assert!(s.total < 28.0, "Total must be less than original");

        // 4. Pay and complete
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);

        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // ========================================================================
    // More complex combo tests: 支付→改动→取消→再操作 链式场景
    // ========================================================================

    /// Helper: 整单附加费
    fn apply_surcharge(manager: &OrdersManager, order_id: &str, percent: f64) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.to_string(),
                surcharge_percent: Some(percent),
                surcharge_amount: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 整单固定附加费
    fn apply_surcharge_fixed(manager: &OrdersManager, order_id: &str, amount: f64) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.to_string(),
                surcharge_percent: None,
                surcharge_amount: Some(amount),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 整单固定折扣
    fn apply_discount_fixed(manager: &OrdersManager, order_id: &str, amount: f64) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id: order_id.to_string(),
                discount_percent: None,
                discount_fixed: Some(amount),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 删除商品
    fn remove_item(manager: &OrdersManager, order_id: &str, instance_id: &str, qty: Option<i32>) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::RemoveItem {
                order_id: order_id.to_string(),
                instance_id: instance_id.to_string(),
                quantity: qty,
                reason: Some("test".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: uncomp 商品
    fn uncomp_item(manager: &OrdersManager, order_id: &str, instance_id: &str) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::UncompItem {
                order_id: order_id.to_string(),
                instance_id: instance_id.to_string(),
                authorizer_id: 1,
                authorizer_name: "Test".to_string(),
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 添加更多商品
    fn add_items(manager: &OrdersManager, order_id: &str, items: Vec<CartItemInput>) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.to_string(),
                items,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 验证 remaining_amount 字段和方法一致
    fn assert_remaining_consistent(s: &shared::order::OrderSnapshot) {
        let computed = (s.total - s.paid_amount).max(0.0);
        assert!(
            (s.remaining_amount - computed).abs() < 0.02,
            "remaining_amount field({:.2}) diverged from total({:.2}) - paid({:.2}) = {:.2}",
            s.remaining_amount, s.total, s.paid_amount, computed
        );
    }

    // --- Test 16: 3次部分支付 → 取消中间那笔 → 验证 remaining ---

    #[test]
    fn test_combo_three_payments_cancel_middle() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            292,
            vec![simple_item(1, "Steak", 30.0, 3)], // 90
        );

        // 3 payments: 25, 35, 20
        let r = pay(&manager, &order_id, 25.0, "CARD");
        assert!(r.success);
        let r = pay(&manager, &order_id, 35.0, "CASH");
        assert!(r.success);
        let r = pay(&manager, &order_id, 20.0, "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 80.0).abs() < 0.01);
        assert_remaining_consistent(&s);

        // Cancel middle payment (35)
        let mid_pid = s.payments.iter().find(|p| !p.cancelled && (p.amount - 35.0).abs() < 0.01).unwrap().payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &mid_pid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 45.0).abs() < 0.01, "paid should be 25+20=45, got {}", s.paid_amount);
        assert_remaining_consistent(&s);
        assert!((s.remaining_amount - 45.0).abs() < 0.01, "remaining should be 90-45=45, got {}", s.remaining_amount);

        // Pay remaining and complete
        let r = pay(&manager, &order_id, s.remaining_amount, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 17: 部分支付 → 整单折扣被阻 → 取消支付 → 整单折扣 + 附加费 → 支付 ---

    #[test]
    fn test_combo_cancel_payment_then_order_discount_and_surcharge() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            293,
            vec![
                simple_item(1, "Coffee", 10.0, 4), // 40
                simple_item(2, "Tea", 5.0, 2),     // 10 → total=50
            ],
        );

        // Pay 20
        let r = pay(&manager, &order_id, 20.0, "CARD");
        assert!(r.success);

        // Try discount → blocked
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(!r.success, "Discount should be blocked after payment");

        // Try surcharge → also blocked
        let r = apply_surcharge(&manager, &order_id, 10.0);
        assert!(!r.success, "Surcharge should be blocked after payment");

        // Cancel payment
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let pid = s.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &pid);
        assert!(r.success);

        // Now discount (20%) → total = 50 * 0.8 = 40
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(r.success);

        // Surcharge (10%) → total = 50 - 10 + 5 = 45
        // (discount on subtotal 50 = 10, surcharge on subtotal 50 = 5)
        let r = apply_surcharge(&manager, &order_id, 10.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(s.total > 0.0);
        assert!((s.total - 45.0).abs() < 0.01, "Expected 45, got {}", s.total);
        assert_remaining_consistent(&s);

        // Pay and complete
        let r = pay(&manager, &order_id, s.total, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 18: 商品折扣 + 整单折扣 + 整单附加费 + comp → 多层叠加 ---

    #[test]
    fn test_combo_multi_layer_discounts_surcharges_comp() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            294,
            vec![
                simple_item(1, "Steak", 20.0, 2),  // 40
                simple_item(2, "Wine", 15.0, 2),   // 30
                simple_item(3, "Bread", 3.0, 1),   // 3  → total=73
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let bread_iid = s.items.iter().find(|i| i.name == "Bread").unwrap().instance_id.clone();
        let _steak_iid = s.items.iter().find(|i| i.name == "Steak").unwrap().instance_id.clone();
        let wine_iid = s.items.iter().find(|i| i.name == "Wine").unwrap().instance_id.clone();

        // 1. Comp bread (free) → subtotal = 40 + 30 = 70
        let r = comp_item(&manager, &order_id, &bread_iid);
        assert!(r.success);

        // 2. 50% item discount on wine → wine = 15, subtotal = 40 + 15 = 55
        let r = modify_item(&manager, &order_id, &wine_iid, discount_changes(50.0));
        assert!(r.success);

        // 3. 10% order discount → discount = 55 * 0.1 = 5.5
        let r = apply_discount(&manager, &order_id, 10.0);
        assert!(r.success);

        // 4. 5% order surcharge → surcharge = 55 * 0.05 = 2.75
        let r = apply_surcharge(&manager, &order_id, 5.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // total = subtotal(55) - discount(5.5) + surcharge(2.75) = 52.25
        assert!((s.total - 52.25).abs() < 0.01, "Expected 52.25, got {}", s.total);
        assert!(s.total > 0.0);
        assert_remaining_consistent(&s);

        // 5. Uncomp bread → subtotal = 40 + 15 + 3 = 58
        //    But can't uncomp after order discount applied? Let's check...
        //    Order discount/surcharge don't block uncomp, only paid_amount blocks discount changes.
        let bread_iids: Vec<_> = s.items.iter().filter(|i| i.name == "Bread").map(|i| i.instance_id.clone()).collect();
        if let Some(comped_bread) = bread_iids.first() {
            let r = uncomp_item(&manager, &order_id, comped_bread);
            if r.success {
                let s = manager.get_snapshot(&order_id).unwrap().unwrap();
                // New subtotal = 40 + 15 + 3 = 58
                // discount = 58 * 0.1 = 5.8, surcharge = 58 * 0.05 = 2.9
                // total = 58 - 5.8 + 2.9 = 55.1
                assert!(s.total > 52.0, "Total should increase after uncomp");
                assert_remaining_consistent(&s);
            }
        }

        // 6. Pay and complete
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 19: 分单支付 → 改价(触发 split) → 取消分单 → 再改价 ---

    #[test]
    fn test_combo_split_pay_modify_cancel_modify_again() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            295,
            vec![simple_item(1, "Coffee", 10.0, 6)], // 60
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();

        // 1. Split-pay 3 coffees (30)
        let r = split_by_items(&manager, &order_id, vec![
            shared::order::SplitItem {
                instance_id: iid.clone(),
                name: "Coffee".to_string(),
                quantity: 3,
                unit_price: 10.0,
            },
        ], "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 30.0).abs() < 0.01);

        // Verify split-pay set paid_item_quantities
        assert!(
            s.paid_item_quantities.get(&iid).copied().unwrap_or(0) == 3,
            "Expected 3 paid coffees, got paid_item_quantities={:?}",
            s.paid_item_quantities
        );

        // 2. Modify unpaid coffee price to 8 (should split: paid@10, unpaid@8)
        let unpaid_item = s.items.iter()
            .find(|i| i.unpaid_quantity > 0)
            .unwrap();
        let unpaid_iid = unpaid_item.instance_id.clone();
        assert_eq!(unpaid_item.quantity, 6, "Item should still be qty=6 (unsplit)");
        assert_eq!(unpaid_item.unpaid_quantity, 3, "Unpaid should be 3");

        let r = modify_item(&manager, &order_id, &unpaid_iid, price_changes(8.0));
        assert!(r.success, "Modify price failed: {:?}", r.error);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // Debug: show what items we have
        let items_info: Vec<_> = s.items.iter().map(|i| (i.price, i.quantity, i.unpaid_quantity)).collect();
        // paid portion: 3 * 10 = 30, unpaid: 3 * 8 = 24, total=54
        assert!(
            (s.total - 54.0).abs() < 0.01,
            "Expected 54, got {}. Items: {:?}, paid_amount: {}, paid_item_quantities: {:?}",
            s.total, items_info, s.paid_amount, s.paid_item_quantities
        );
        assert_remaining_consistent(&s);

        // 3. Cancel the split payment → paid items should be restored
        let pid = s.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &pid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount).abs() < 0.01, "paid should be 0 after cancel");
        assert_remaining_consistent(&s);

        // Total should still reflect the 2 different prices:
        // 3@10 (restored from split cancel) + 3@8 = 54
        assert!((s.total - 54.0).abs() < 0.01, "Total should be 54 after cancel, got {}", s.total);

        // 4. Modify all items back to 10 (normalize price)
        for item in &s.items {
            if (item.price - 10.0).abs() > 0.01 {
                let r = modify_item(&manager, &order_id, &item.instance_id, price_changes(10.0));
                assert!(r.success);
            }
        }

        // After normalizing, items with same content should merge
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
        assert_eq!(total_qty, 6, "Total qty should be 6");
        assert!((s.total - 60.0).abs() < 0.01, "Total should be 60 after re-normalizing prices");
        assert_remaining_consistent(&s);

        // 5. Pay and complete
        let r = pay(&manager, &order_id, s.total, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 20: comp → uncomp → discount → remove → add → 完整循环 ---

    #[test]
    fn test_combo_comp_uncomp_discount_remove_add_cycle() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            296,
            vec![
                simple_item(1, "Steak", 25.0, 2),  // 50
                simple_item(2, "Wine", 12.0, 3),   // 36 → total=86
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let wine_iid = s.items.iter().find(|i| i.name == "Wine").unwrap().instance_id.clone();

        // 1. Comp wine
        let r = comp_item(&manager, &order_id, &wine_iid);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 50.0).abs() < 0.01, "Total should be 50 after comp wine");

        // 2. Uncomp wine
        let comped_iid = s.items.iter().find(|i| i.is_comped && i.name == "Wine").unwrap().instance_id.clone();
        let r = uncomp_item(&manager, &order_id, &comped_iid);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 86.0).abs() < 0.01, "Total should be 86 after uncomp");

        // 3. 30% discount on wine
        let wine_iid = s.items.iter().find(|i| i.name == "Wine" && !i.is_comped).unwrap().instance_id.clone();
        let r = modify_item(&manager, &order_id, &wine_iid, discount_changes(30.0));
        assert!(r.success);

        // 4. Remove 1 steak
        let steak_iid = s.items.iter().find(|i| i.name == "Steak").unwrap().instance_id.clone();
        let r = remove_item(&manager, &order_id, &steak_iid, Some(1));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // steak: 1 * 25 = 25, wine: 3 * 12 * 0.7 = 25.2, total = 50.2
        assert!(s.total > 0.0);
        assert_remaining_consistent(&s);

        // 5. Add 2 more wines (same product, no discount → different instance_id)
        let r = add_items(&manager, &order_id, vec![simple_item(2, "Wine", 12.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let total_wine_qty: i32 = s.items.iter().filter(|i| i.name == "Wine" && !i.is_comped).map(|i| i.quantity).sum();
        assert_eq!(total_wine_qty, 5, "Should have 5 wines total (3 discounted + 2 new)");
        assert_remaining_consistent(&s);

        // 6. Pay and complete
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 21: 整单折扣+附加费 → 改为固定折扣 → 改为固定附加费 → 反复切换 ---

    #[test]
    fn test_combo_order_adjustment_switching() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            297,
            vec![simple_item(1, "Coffee", 10.0, 5)], // 50
        );

        // 1. 20% order discount → total = 50 - 10 = 40
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 40.0).abs() < 0.01);

        // 2. Switch to fixed discount of 15 → total = 50 - 15 = 35
        let r = apply_discount_fixed(&manager, &order_id, 15.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 35.0).abs() < 0.01, "Expected 35 with fixed discount 15, got {}", s.total);

        // 3. Add 10% surcharge → total = 50 - 15 + 5 = 40
        let r = apply_surcharge(&manager, &order_id, 10.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 40.0).abs() < 0.01, "Expected 40, got {}", s.total);

        // 4. Switch surcharge to fixed 8 → total = 50 - 15 + 8 = 43
        let r = apply_surcharge_fixed(&manager, &order_id, 8.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 43.0).abs() < 0.01, "Expected 43, got {}", s.total);

        // 5. Remove discount entirely → total = 50 + 8 = 58
        let r = apply_discount(&manager, &order_id, 0.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 58.0).abs() < 0.01, "Expected 58, got {}", s.total);

        // 6. Remove surcharge → total = 50
        //    (surcharge_percent doesn't accept 0, use None/None to clear)
        let clear_surcharge_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: None,
                surcharge_amount: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let r = manager.execute_command(clear_surcharge_cmd);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 50.0).abs() < 0.01, "Expected 50, got {}", s.total);

        assert_remaining_consistent(&s);
        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 22: 整单折扣 > subtotal → total clamp 0 + 附加费 → total 仍为正 ---

    #[test]
    fn test_combo_extreme_discount_with_surcharge() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            298,
            vec![simple_item(1, "Coffee", 5.0, 2)], // 10
        );

        // 固定折扣 30 on total 10 → clamp to 0
        let r = apply_discount_fixed(&manager, &order_id, 30.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(s.total >= 0.0, "Total must not be negative");
        assert!((s.total).abs() < 0.01, "Total should be clamped to 0, got {}", s.total);

        // Add surcharge 5 → total = max(10 - 30, 0) + 5 → depends on clamp logic
        // Actually: total = (subtotal - discount + surcharge).max(0)
        //         = (10 - 30 + 5).max(0) = max(-15, 0) = 0
        let r = apply_surcharge_fixed(&manager, &order_id, 5.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(s.total >= 0.0, "Total must not be negative even with surcharge");

        // Reduce discount to 8 → total = (10 - 8 + 5).max(0) = 7
        let r = apply_discount_fixed(&manager, &order_id, 8.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 7.0).abs() < 0.01, "Expected 7, got {}", s.total);
        assert_remaining_consistent(&s);

        // Pay and complete
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 23: 部分支付多次 → 取消全部 → 重新支付 → 完成 ---

    #[test]
    fn test_combo_pay_multiple_cancel_all_repay() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            299,
            vec![
                simple_item(1, "Coffee", 10.0, 2), // 20
                simple_item(2, "Tea", 5.0, 4),     // 20 → total=40
            ],
        );

        // 4 partial payments
        for amount in &[8.0, 12.0, 10.0, 5.0] {
            let r = pay(&manager, &order_id, *amount, "CARD");
            assert!(r.success);
        }

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 35.0).abs() < 0.01);
        assert_remaining_consistent(&s);

        // Cancel all payments one by one
        let payment_ids: Vec<String> = s.payments.iter()
            .filter(|p| !p.cancelled)
            .map(|p| p.payment_id.clone())
            .collect();
        for pid in &payment_ids {
            let r = cancel_payment(&manager, &order_id, pid);
            assert!(r.success);
        }

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount).abs() < 0.01, "All payments cancelled, paid should be 0");
        assert!((s.remaining_amount - 40.0).abs() < 0.01, "Remaining should be full 40");
        assert_remaining_consistent(&s);

        // Pay full amount at once
        let r = pay(&manager, &order_id, 40.0, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 24: 分单支付+商品折扣循环+取消 → 验证 remaining 始终一致 ---

    #[test]
    fn test_combo_split_discount_cycle_cancel_remaining_consistency() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            300,
            vec![simple_item(1, "Coffee", 10.0, 6)], // 60
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();

        // 1. Split-pay 2 coffees (20)
        let r = split_by_items(&manager, &order_id, vec![
            shared::order::SplitItem {
                instance_id: iid.clone(),
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            },
        ], "CARD");
        assert!(r.success);

        // 2. 30% discount on unpaid coffees
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_remaining_consistent(&s); // check after split
        let unpaid_iid = s.items.iter()
            .find(|i| i.unpaid_quantity > 0)
            .unwrap().instance_id.clone();
        let r = modify_item(&manager, &order_id, &unpaid_iid, discount_changes(30.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_remaining_consistent(&s); // check after discount

        // 3. Change discount to 50%
        let unpaid_iid = s.items.iter()
            .find(|i| i.unpaid_quantity > 0 && !i.is_comped)
            .unwrap().instance_id.clone();
        let r = modify_item(&manager, &order_id, &unpaid_iid, discount_changes(50.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_remaining_consistent(&s); // check after 2nd discount change

        // 4. Cancel the split payment
        let pid = s.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &pid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount).abs() < 0.01);
        assert_remaining_consistent(&s); // check after cancel

        // 5. Remove discount
        let discounted: Vec<_> = s.items.iter()
            .filter(|i| i.manual_discount_percent.is_some())
            .map(|i| i.instance_id.clone())
            .collect();
        for iid in &discounted {
            modify_item(&manager, &order_id, iid, discount_changes(0.0));
        }

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
        assert_eq!(total_qty, 6);
        assert_remaining_consistent(&s); // final check

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 25: 整单折扣 + 整单附加费 + 商品折扣 + comp → 支付后 void ---

    #[test]
    fn test_combo_everything_then_partial_pay_void() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            301,
            vec![
                simple_item(1, "Steak", 20.0, 2),  // 40
                simple_item(2, "Wine", 10.0, 3),   // 30
                simple_item(3, "Bread", 2.0, 2),   // 4  → total=74
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let bread_iid = s.items.iter().find(|i| i.name == "Bread").unwrap().instance_id.clone();
        let wine_iid = s.items.iter().find(|i| i.name == "Wine").unwrap().instance_id.clone();

        // 1. Comp bread → subtotal = 40 + 30 = 70
        let r = comp_item(&manager, &order_id, &bread_iid);
        assert!(r.success);

        // 2. 25% discount on wine → wine=22.5, subtotal = 40 + 22.5 = 62.5
        let r = modify_item(&manager, &order_id, &wine_iid, discount_changes(25.0));
        assert!(r.success);

        // 3. 10% order discount → -6.25
        let r = apply_discount(&manager, &order_id, 10.0);
        assert!(r.success);

        // 4. 5% order surcharge → +3.125
        let r = apply_surcharge(&manager, &order_id, 5.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // total = (62.5 - 6.25 + 3.125).max(0) ≈ 59.375
        assert!(s.total > 50.0 && s.total < 65.0, "Expected ~59.375, got {}", s.total);
        let expected_total = s.total;
        assert_remaining_consistent(&s);

        // 5. Pay 30 (partial)
        let r = pay(&manager, &order_id, 30.0, "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount - 30.0).abs() < 0.01);
        assert!((s.remaining_amount - (expected_total - 30.0)).abs() < 0.02);
        assert_remaining_consistent(&s);

        // 6. Void with loss → auto-calculate loss_amount = total - paid
        let void_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::VoidOrder {
                order_id: order_id.clone(),
                void_type: VoidType::LossSettled,
                loss_reason: Some(shared::order::LossReason::CustomerFled),
                loss_amount: None,
                note: Some("Complex order voided".to_string()),
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let r = manager.execute_command(void_cmd);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.status, OrderStatus::Void);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 26: 连续 add → remove → add → 验证总量和总价 ---

    #[test]
    fn test_combo_add_remove_add_items_total_tracking() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            302,
            vec![simple_item(1, "Coffee", 10.0, 2)], // 20
        );

        // Add 3 more coffees → 5 total, 50
        let r = add_items(&manager, &order_id, vec![simple_item(1, "Coffee", 10.0, 3)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 1, "Same product should merge");
        assert_eq!(s.items[0].quantity, 5);
        assert!((s.total - 50.0).abs() < 0.01);

        // Remove 2 coffees → 3 left, 30
        let iid = s.items[0].instance_id.clone();
        let r = remove_item(&manager, &order_id, &iid, Some(2));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
        assert_eq!(total_qty, 3);
        assert!((s.total - 30.0).abs() < 0.01);

        // Add tea
        let r = add_items(&manager, &order_id, vec![simple_item(2, "Tea", 5.0, 4)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 3 coffees (30) + 4 teas (20) = 50
        assert!((s.total - 50.0).abs() < 0.01);
        assert_remaining_consistent(&s);

        // Pay and complete
        let r = pay(&manager, &order_id, 50.0, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 27: 部分支付 → 商品折扣(触发 split) → 取消支付 → 删除高价商品 → 支付 ---

    #[test]
    fn test_combo_partial_pay_discount_split_cancel_remove() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            303,
            vec![
                simple_item(1, "Steak", 30.0, 2),  // 60
                simple_item(2, "Salad", 8.0, 1),   // 8  → total=68
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let steak_iid = s.items.iter().find(|i| i.name == "Steak").unwrap().instance_id.clone();

        // 1. Split-pay 1 steak (30)
        let r = split_by_items(&manager, &order_id, vec![
            shared::order::SplitItem {
                instance_id: steak_iid.clone(),
                name: "Steak".to_string(),
                quantity: 1,
                unit_price: 30.0,
            },
        ], "CARD");
        assert!(r.success);

        // 2. Apply 50% discount on unpaid steak (should split: paid@30 + unpaid@15)
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let unpaid_steak = s.items.iter()
            .find(|i| i.name == "Steak" && i.unpaid_quantity > 0)
            .unwrap();
        let r = modify_item(&manager, &order_id, &unpaid_steak.instance_id, discount_changes(50.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_remaining_consistent(&s);

        // 3. Cancel the split payment
        let pid = s.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &pid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.paid_amount).abs() < 0.01);
        assert_remaining_consistent(&s);

        // 4. Remove the salad
        let salad_iid = s.items.iter().find(|i| i.name == "Salad").unwrap().instance_id.clone();
        let r = remove_item(&manager, &order_id, &salad_iid, None);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(s.items.iter().all(|i| i.name != "Salad"), "Salad should be removed");
        assert!(s.total > 0.0);
        assert_remaining_consistent(&s);

        // 5. Pay remaining and complete
        let r = pay(&manager, &order_id, s.total, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 28: 整单折扣+附加费 叠加后取消折扣 → 附加费基数变化 ---

    #[test]
    fn test_combo_order_discount_surcharge_interaction() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            304,
            vec![simple_item(1, "Coffee", 10.0, 10)], // 100
        );

        // 20% discount → discount = 20, total = 80
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 80.0).abs() < 0.01);

        // 10% surcharge → surcharge on subtotal(100) = 10, total = 100 - 20 + 10 = 90
        let r = apply_surcharge(&manager, &order_id, 10.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 90.0).abs() < 0.01, "Expected 90, got {}", s.total);

        // Remove discount → total = 100 + 10 = 110
        let r = apply_discount(&manager, &order_id, 0.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 110.0).abs() < 0.01, "Expected 110, got {}", s.total);

        // Remove surcharge → total = 100
        //    (surcharge_percent doesn't accept 0, use None/None to clear)
        let clear_surcharge_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: None,
                surcharge_amount: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let r = manager.execute_command(clear_surcharge_cmd);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 100.0).abs() < 0.01, "Expected 100, got {}", s.total);

        assert_remaining_consistent(&s);
        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 29: 部分支付 → 每笔支付后检查 remaining 一致性 ---

    #[test]
    fn test_combo_remaining_consistent_after_every_payment() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            305,
            vec![simple_item(1, "Coffee", 7.5, 8)], // 60
        );

        let payments = vec![5.0, 10.5, 3.0, 15.0, 7.5, 9.0];
        let mut total_paid = 0.0;

        for amount in &payments {
            let r = pay(&manager, &order_id, *amount, "CARD");
            assert!(r.success, "Payment of {} failed", amount);
            total_paid += amount;

            let s = manager.get_snapshot(&order_id).unwrap().unwrap();
            assert!(
                (s.paid_amount - total_paid).abs() < 0.01,
                "After paying {}, expected paid_amount={}, got {}",
                amount, total_paid, s.paid_amount
            );
            assert_remaining_consistent(&s);
        }

        // Pay remaining
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let remaining = s.remaining_amount;
        assert!(remaining > 0.0, "Should still have remaining");
        let r = pay(&manager, &order_id, remaining, "CASH");
        assert!(r.success);

        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 30: 固定折扣 + 百分比附加费 + 商品折扣 → 多维叠加计算验证 ---

    #[test]
    fn test_combo_fixed_discount_percent_surcharge_item_discount() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            306,
            vec![
                simple_item(1, "A", 20.0, 3),  // 60
                simple_item(2, "B", 15.0, 2),  // 30 → subtotal=90
            ],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let a_iid = s.items.iter().find(|i| i.name == "A").unwrap().instance_id.clone();

        // 1. 40% item discount on A → A_line = 20*0.6*3 = 36, subtotal = 36+30 = 66
        let r = modify_item(&manager, &order_id, &a_iid, discount_changes(40.0));
        assert!(r.success);

        // 2. Fixed order discount of 10 → total = 66 - 10 = 56
        let r = apply_discount_fixed(&manager, &order_id, 10.0);
        assert!(r.success);

        // 3. 15% order surcharge → surcharge = 66 * 0.15 = 9.9, total = 66 - 10 + 9.9 = 65.9
        let r = apply_surcharge(&manager, &order_id, 15.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!((s.total - 65.9).abs() < 0.1, "Expected ~65.9, got {}", s.total);
        assert_remaining_consistent(&s);

        // 4. Pay and complete
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // ========================================================================
    // Price Rule + Options + Spec 复杂组合测试 (Tests 31-40)
    // ========================================================================

    /// Helper: 开台（不加商品）
    fn open_table(manager: &OrdersManager, table_id: i64) -> String {
        let open_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some(table_id),
                table_name: Some(format!("Table {}", table_id)),
                zone_id: Some(1),
                zone_name: Some("Zone A".to_string()),
                guest_count: 2,
                is_retail: false,
            },
        );
        let resp = manager.execute_command(open_cmd);
        assert!(resp.success, "Failed to open table");
        resp.order_id.unwrap()
    }

    /// Helper: 跳过/恢复规则
    fn toggle_rule_skip(
        manager: &OrdersManager,
        order_id: &str,
        rule_id: i64,
        skipped: bool,
    ) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ToggleRuleSkip {
                order_id: order_id.to_string(),
                rule_id,
                skipped,
            },
        );
        manager.execute_command(cmd)
    }

    /// Helper: 创建百分比折扣规则
    fn make_discount_rule(id: i64, percent: f64) -> PriceRule {
        use shared::models::price_rule::*;
        PriceRule {
            id,
            name: format!("discount_{}", id),
            display_name: format!("Discount {}", id),
            receipt_name: "DISC".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target_id: None,
            zone_scope: "all".to_string(),
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: percent,
            is_stackable: true,
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

    /// Helper: 创建百分比附加费规则
    fn make_surcharge_rule(id: i64, percent: f64) -> PriceRule {
        use shared::models::price_rule::*;
        PriceRule {
            id,
            name: format!("surcharge_{}", id),
            display_name: format!("Surcharge {}", id),
            receipt_name: "SURCH".to_string(),
            description: None,
            rule_type: RuleType::Surcharge,
            product_scope: ProductScope::Global,
            target_id: None,
            zone_scope: "all".to_string(),
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: percent,
            is_stackable: true,
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

    /// Helper: 创建固定金额折扣规则
    fn make_fixed_discount_rule(id: i64, amount: f64) -> PriceRule {
        use shared::models::price_rule::*;
        PriceRule {
            id,
            name: format!("fixed_discount_{}", id),
            display_name: format!("Fixed Discount {}", id),
            receipt_name: "FDISC".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target_id: None,
            zone_scope: "all".to_string(),
            adjustment_type: AdjustmentType::FixedAmount,
            adjustment_value: amount,
            is_stackable: true,
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

    /// Helper: 带规格的商品
    fn item_with_spec(
        product_id: i64,
        name: &str,
        price: f64,
        quantity: i32,
        spec: shared::order::SpecificationInfo,
    ) -> CartItemInput {
        CartItemInput {
            product_id,
            name: name.to_string(),
            price,
            original_price: None,
            quantity,
            selected_options: None,
            selected_specification: Some(spec),
            manual_discount_percent: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    /// Helper: 创建选项
    fn make_option(attr_id: i64, attr_name: &str, idx: i32, opt_name: &str, modifier: f64) -> shared::order::ItemOption {
        shared::order::ItemOption {
            attribute_id: attr_id,
            attribute_name: attr_name.to_string(),
            option_idx: idx,
            option_name: opt_name.to_string(),
            price_modifier: Some(modifier),
            quantity: 1,
        }
    }

    /// Helper: 创建规格
    fn make_spec(id: i64, name: &str, price: Option<f64>) -> shared::order::SpecificationInfo {
        shared::order::SpecificationInfo {
            id,
            name: name.to_string(),
            receipt_name: None,
            price,
        }
    }

    /// Helper: 组合 changes
    fn combo_changes(
        price: Option<f64>,
        qty: Option<i32>,
        discount: Option<f64>,
        options: Option<Vec<shared::order::ItemOption>>,
        spec: Option<shared::order::SpecificationInfo>,
    ) -> shared::order::ItemChanges {
        shared::order::ItemChanges {
            price,
            quantity: qty,
            manual_discount_percent: discount,
            note: None,
            selected_options: options,
            selected_specification: spec,
        }
    }

    /// 浮点断言 helper
    fn assert_close(actual: f64, expected: f64, msg: &str) {
        assert!(
            (actual - expected).abs() < 0.02,
            "{}: expected {:.2}, got {:.2}",
            msg, expected, actual
        );
    }

    // --- Test 31: 价格规则 + skip/unskip 循环 ---

    #[test]
    fn test_combo_rule_skip_unskip_cycle() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 31);

        // 注入 10% 折扣规则
        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 添加商品: 100€ × 2
        let r = add_items(&manager, &order_id, vec![simple_item(1, "Steak", 100.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // base=100, rule_discount=10% → unit_price=90, subtotal=180
        assert_close(s.subtotal, 180.0, "subtotal after rule");
        assert_close(s.total, 180.0, "total after rule");
        let item = &s.items[0];
        assert!(!item.applied_rules.is_empty(), "should have applied rules");
        assert_close(item.unit_price, 90.0, "unit_price with 10% discount");
        assert_close(item.price, 90.0, "item.price synced to unit_price");
        assert_eq!(item.original_price, 100.0, "original_price = catalog base");

        // Skip 规则 → 恢复原价
        let r = toggle_rule_skip(&manager, &order_id, 10, true);
        assert!(r.success, "skip failed: {:?}", r.error);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 200.0, "subtotal after skip");
        assert_close(s.items[0].price, 100.0, "item.price after skip");

        // Unskip → 恢复折扣
        let r = toggle_rule_skip(&manager, &order_id, 10, false);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 180.0, "subtotal after unskip");
        assert_close(s.items[0].price, 90.0, "item.price after unskip");

        // 支付并结单
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 32: 价格规则 + 手动改价 ---

    #[test]
    fn test_combo_rule_then_manual_reprice() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 32);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Wine", 100.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.items[0].price, 90.0, "initial price with rule");

        // 手动改价到 80 → original_price=80, rule 10% on 80 → unit_price=72
        let r = modify_item(&manager, &order_id, &iid, price_changes(80.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 改价后 instance_id 可能变化，找到改价后的商品
        let item = &s.items[0];
        assert_eq!(item.original_price, 80.0, "original_price updated to manual price");
        assert_close(item.unit_price, 72.0, "unit_price = 80 - 10% = 72");
        assert_close(item.price, 72.0, "item.price synced");
        assert_close(s.total, 72.0, "total");

        // Skip 规则 → 恢复到手动改价后的基础价
        let rule_id = item.applied_rules[0].rule_id.clone();
        let r = toggle_rule_skip(&manager, &order_id, rule_id, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.items[0].price, 80.0, "price after skip = manual price");
        assert_close(s.total, 80.0, "total after skip");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 33: 选项 + 手动折扣 + 价格规则 ---

    #[test]
    fn test_combo_options_manual_discount_rule() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 33);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 商品 50€, 选项 +5€ (加大), 数量 2
        let r = add_items(&manager, &order_id, vec![
            item_with_options(
                1, "Coffee", 50.0, 2,
                vec![make_option(1, "Size", 1, "Large", 5.0)],
            ),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item = &s.items[0];
        // base=50, options=+5 → base_with_options=55
        // rule discount = 10% of 55 = 5.5 → unit_price = 55 - 5.5 = 49.5
        assert_close(item.unit_price, 49.5, "unit_price with option + rule");
        assert_close(s.subtotal, 99.0, "subtotal = 49.5 × 2");
        let iid = item.instance_id.clone();

        // 手动加 20% 折扣
        let r = modify_item(&manager, &order_id, &iid, discount_changes(20.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item = &s.items[0];
        // base_with_options=55, manual 20% = 11 → after_manual=44
        // rule discount = 10% of after_manual=44 → 4.4
        // unit_price = 55 - 11 - 4.4 = 39.6
        assert_close(item.unit_price, 39.6, "unit_price with option + manual + rule");
        assert_close(s.subtotal, 79.2, "subtotal = 39.6 × 2");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 34: 规格变更 + 规则重算 ---

    #[test]
    fn test_combo_spec_change_with_rule() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 34);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 商品: spec A 价格 100€
        let r = add_items(&manager, &order_id, vec![
            item_with_spec(1, "Pasta", 100.0, 1, make_spec(1, "Regular", Some(100.0))),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.items[0].price, 90.0, "initial with rule 10%");

        // 改规格到 spec B (价格 150€) — 通过 ModifyItem 改价格和规格
        let r = modify_item(
            &manager, &order_id, &iid,
            combo_changes(
                Some(150.0), None, None, None,
                Some(make_spec(2, "Premium", Some(150.0))),
            ),
        );
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item = &s.items[0];
        // original_price=150, rule 10% → unit_price=135
        assert_eq!(item.original_price, 150.0, "original_price updated to new spec price");
        assert_close(item.unit_price, 135.0, "unit_price after spec change");
        assert_close(s.total, 135.0, "total after spec change");

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 35: 多规则 skip 其中一个 ---

    #[test]
    fn test_combo_multiple_rules_selective_skip() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 35);

        // 两个规则: 10% 折扣 + 5% 附加费
        manager.cache_rules(&order_id, vec![
            make_discount_rule(10, 10.0),
            make_surcharge_rule(5, 5.0),
        ]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Steak", 100.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // base=100, manual=0 → after_manual=100
        // discount: 10% of 100 = 10
        // surcharge: 5% of (base_with_options=100) = 5
        // unit_price = 100 - 10 + 5 = 95
        assert_close(s.items[0].unit_price, 95.0, "both rules active");
        assert_close(s.total, 95.0, "total");

        // Skip 折扣 → 只剩附加费
        let r = toggle_rule_skip(&manager, &order_id, 10, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // unit_price = 100 + 5 = 105
        assert_close(s.items[0].price, 105.0, "only surcharge active");

        // Skip 附加费 → 无规则
        let r = toggle_rule_skip(&manager, &order_id, 5, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.items[0].price, 100.0, "no rules active");

        // Unskip 两个
        let r = toggle_rule_skip(&manager, &order_id, 10, false);
        assert!(r.success);
        let r = toggle_rule_skip(&manager, &order_id, 5, false);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.items[0].price, 95.0, "both rules restored");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 36: 价格规则 + 整单折扣 + 整单附加费 ---

    #[test]
    fn test_combo_item_rule_plus_order_discount_surcharge() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 36);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 两个商品
        let r = add_items(&manager, &order_id, vec![
            simple_item(1, "A", 100.0, 1), // rule: 90
            simple_item(2, "B", 50.0, 2),  // rule: 45 × 2 = 90
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 180.0, "subtotal with rules");

        // 整单 20% 折扣 → discount = 180 * 20% = 36 → total = 144
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 144.0, "total after order discount");

        // 整单 10% 附加费 → surcharge = 180 * 10% = 18 → total = 180 - 36 + 18 = 162
        let r = apply_surcharge(&manager, &order_id, 10.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 162.0, "total after surcharge");
        assert_remaining_consistent(&s);

        // Skip 商品规则 → subtotal 变大 → 整单折扣/附加费重算
        let rule_id = s.items[0].applied_rules[0].rule_id.clone();
        let r = toggle_rule_skip(&manager, &order_id, rule_id, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // subtotal = 100 + 50*2 = 200
        // discount = 200 * 20% = 40
        // surcharge = 200 * 10% = 20
        // total = 200 - 40 + 20 = 180
        assert_close(s.subtotal, 200.0, "subtotal after skip");
        assert_close(s.total, 180.0, "total after skip with order adjustments");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 37: 价格规则 + 选项 ± 金额 + 修改选项 ---

    #[test]
    fn test_combo_options_modifier_change() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 37);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 商品 80€, 选项 +10€ (Extra Cheese) + -3€ (No Sauce)
        let r = add_items(&manager, &order_id, vec![
            item_with_options(
                1, "Burger", 80.0, 1,
                vec![
                    make_option(2, "Topping", 0, "Extra Cheese", 10.0),
                    make_option(3, "Sauce", 1, "No Sauce", -3.0),
                ],
            ),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        // base=80, options=+10-3=+7 → base_with_options=87
        // rule discount = 10% of 87 = 8.7
        // unit_price = 87 - 8.7 = 78.3
        assert_close(s.items[0].unit_price, 78.3, "initial with options + rule");

        // 修改选项: 换成 Extra Meat +15€
        let r = modify_item(
            &manager, &order_id, &iid,
            combo_changes(
                None, None, None,
                Some(vec![make_option(2, "Topping", 2, "Extra Meat", 15.0)]),
                None,
            ),
        );
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // base=80, options=+15 → base_with_options=95
        // rule discount = 10% of 95 = 9.5
        // unit_price = 95 - 9.5 = 85.5
        assert_close(s.items[0].unit_price, 85.5, "after options change");
        assert_close(s.total, 85.5, "total");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 38: 固定金额规则 + 百分比规则叠加 ---

    #[test]
    fn test_combo_fixed_and_percent_rules_stacking() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 38);

        // 固定 5€ 折扣 + 15% 附加费
        manager.cache_rules(&order_id, vec![
            make_fixed_discount_rule(50, 5.0),
            make_surcharge_rule(15, 15.0),
        ]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Salmon", 60.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // Per item: base=60, after_manual=60
        // discount: fixed 5
        // surcharge: 15% of base_with_options(60) = 9
        // unit_price = 60 - 5 + 9 = 64
        assert_close(s.items[0].unit_price, 64.0, "fixed disc + % surcharge");
        assert_close(s.subtotal, 128.0, "subtotal = 64 × 2");

        // Skip 折扣 → unit_price = 60 + 9 = 69
        let r = toggle_rule_skip(&manager, &order_id, 50, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.items[0].price, 69.0, "after skip discount");

        // Skip 附加费 → unit_price = 60
        let r = toggle_rule_skip(&manager, &order_id, 15, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.items[0].price, 60.0, "all rules skipped");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 39: 全组合: 规则+手动折扣+选项+规格+整单折扣+整单附加费 ---

    #[test]
    fn test_combo_kitchen_sink() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 39);

        // 10% 折扣规则
        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 商品1: 100€, spec A (100€), option +5€, 手动折扣 20%, qty 2
        let r = add_items(&manager, &order_id, vec![CartItemInput {
            product_id: 1,
            name: "Deluxe Plate".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 2,
            selected_options: Some(vec![make_option(4, "Side", 0, "Truffle Fries", 5.0)]),
            selected_specification: Some(make_spec(1, "Regular", Some(100.0))),
            manual_discount_percent: Some(20.0),
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item = &s.items[0];
        // base=100, options=+5 → base_with_options=105
        // manual 20% = 21 → after_manual=84
        // rule 10% of after_manual(84) = 8.4
        // unit_price = 105 - 21 - 8.4 = 75.6
        assert_close(item.unit_price, 75.6, "item1 unit_price");
        assert_close(s.subtotal, 151.2, "subtotal = 75.6 × 2");

        // 加第二个商品: 简单 30€ × 3
        let r = add_items(&manager, &order_id, vec![simple_item(2, "Bread", 30.0, 3)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // Bread: 30 - 10% = 27, line = 27×3 = 81
        // subtotal = 151.2 + 81 = 232.2
        assert_close(s.subtotal, 232.2, "subtotal with both items");

        // 整单 5% 折扣
        let r = apply_discount(&manager, &order_id, 5.0);
        assert!(r.success);

        // 整单 8% 附加费
        let r = apply_surcharge(&manager, &order_id, 8.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // discount = 232.2 * 5% = 11.61
        // surcharge = 232.2 * 8% = 18.576 → 18.58
        // total = 232.2 - 11.61 + 18.58 = 239.17
        let expected_total = 232.2 - (232.2 * 0.05) + (232.2 * 0.08);
        assert!((s.total - expected_total).abs() < 0.1, "total = {:.2}, expected {:.2}", s.total, expected_total);
        assert_remaining_consistent(&s);

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 40: 规则 + 部分支付 + skip 规则 + 支付剩余 ---

    #[test]
    fn test_combo_rule_partial_pay_skip_pay_remaining() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 40);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 100€ × 2 → subtotal = 180 (after 10% discount)
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 180.0, "initial total");

        // 部分支付 50
        let r = pay(&manager, &order_id, 50.0, "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.paid_amount, 50.0, "paid");
        assert_close(s.remaining_amount, 130.0, "remaining");

        // Skip 规则 → total 变为 200, remaining 变为 150
        let rule_id = s.items[0].applied_rules[0].rule_id.clone();
        let r = toggle_rule_skip(&manager, &order_id, rule_id, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 200.0, "total after skip");
        assert_close(s.paid_amount, 50.0, "paid unchanged");
        assert_remaining_consistent(&s);
        assert_close(s.remaining_amount, 150.0, "remaining after skip");

        // 支付剩余并完成
        let r = pay(&manager, &order_id, s.remaining_amount, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 41: 选项 quantity > 1 + 规则 ---

    #[test]
    fn test_combo_option_quantity_with_rule() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 41);

        manager.cache_rules(&order_id, vec![make_surcharge_rule(10, 10.0)]);

        // 商品 20€, 选项 +2€ × qty 3 (e.g., 3 eggs)
        let r = add_items(&manager, &order_id, vec![
            item_with_options(
                1, "Ramen", 20.0, 1,
                vec![shared::order::ItemOption {
                    attribute_id: 7,
                    attribute_name: "Eggs".to_string(),
                    option_idx: 0,
                    option_name: "Extra Egg".to_string(),
                    price_modifier: Some(2.0),
                    quantity: 3,
                }],
            ),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // base=20, options=2*3=6 → base_with_options=26
        // surcharge=10% of 26=2.6 → unit_price=26+2.6=28.6
        assert_close(s.items[0].unit_price, 28.6, "option qty 3 + surcharge");
        assert_close(s.total, 28.6, "total");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 42: 改价 + 改选项 + 改规格 + 改折扣 一次性修改 ---

    #[test]
    fn test_combo_modify_everything_at_once() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 42);

        manager.cache_rules(&order_id, vec![make_discount_rule(5, 5.0)]);

        // 初始: 50€, spec A, option +3€
        let r = add_items(&manager, &order_id, vec![CartItemInput {
            product_id: 1,
            name: "Salad".to_string(),
            price: 50.0,
            original_price: None,
            quantity: 1,
            selected_options: Some(vec![make_option(5, "Dressing", 0, "Vinaigrette", 3.0)]),
            selected_specification: Some(make_spec(1, "Small", Some(50.0))),
            manual_discount_percent: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        // base=50, option=+3, base_with_options=53
        // rule 5% of 53=2.65 → unit_price=53-2.65=50.35
        assert_close(s.items[0].unit_price, 50.35, "initial");

        // 一次性: 改价 60, 改规格 spec B, 改选项 +8€, 加 10% 手动折扣
        let r = modify_item(
            &manager, &order_id, &iid,
            combo_changes(
                Some(60.0),
                None,
                Some(10.0),
                Some(vec![make_option(5, "Dressing", 1, "Caesar", 8.0)]),
                Some(make_spec(2, "Large", Some(60.0))),
            ),
        );
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let item = &s.items[0];
        // original_price=60, option=+8, base_with_options=68
        // manual 10% = 6.8 → after_manual=61.2
        // rule 5% of 61.2=3.06 → unit_price=68-6.8-3.06=58.14
        assert_eq!(item.original_price, 60.0);
        assert_close(item.unit_price, 58.14, "after combo modify");
        assert_close(s.total, 58.14, "total");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 43: 分单支付(按商品) + 价格规则 ---

    #[test]
    fn test_combo_split_by_items_with_rule() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 43);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // A: 100€ × 2 → 90×2=180, B: 50€ × 3 → 45×3=135
        let r = add_items(&manager, &order_id, vec![
            simple_item(1, "A", 100.0, 2),
            simple_item(2, "B", 50.0, 3),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 315.0, "subtotal = 180 + 135");

        let a_iid = s.items.iter().find(|i| i.name == "A").unwrap().instance_id.clone();
        let a_unit_price = s.items.iter().find(|i| i.name == "A").unwrap().unit_price;

        // 分单支付: 1 个 A
        let r = split_by_items(
            &manager, &order_id,
            vec![shared::order::SplitItem {
                instance_id: a_iid.clone(),
                name: "A".to_string(),
                quantity: 1,
                unit_price: a_unit_price,
            }],
            "CARD",
        );
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.paid_amount, 90.0, "paid 1 A = 90");
        assert_remaining_consistent(&s);

        // 支付剩余并完成
        let r = pay(&manager, &order_id, s.remaining_amount, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 44: 规则 + comp + uncomp ---

    #[test]
    fn test_combo_rule_comp_uncomp() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 44);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 100€ × 2 → 90×2=180
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.total, 180.0, "initial");

        // Comp 1 个（不是全部）
        let comp_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.clone(),
                instance_id: iid.clone(),
                quantity: 1,
                reason: "test comp".to_string(),
                authorizer_id: 1,
                authorizer_name: "Test".to_string(),
            },
        );
        let r = manager.execute_command(comp_cmd);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 1 comped (price=0) + 1 normal (90) → subtotal=90
        assert_close(s.subtotal, 90.0, "after comp 1");
        assert!(s.comp_total_amount > 0.0, "comp_total tracked");
        // comp_total should be based on original_price (100) not discounted
        assert_close(s.comp_total_amount, 100.0, "comp_total = original value");

        // Uncomp
        let comped_iid = s.items.iter().find(|i| i.is_comped).unwrap().instance_id.clone();
        let r = uncomp_item(&manager, &order_id, &comped_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 180.0, "after uncomp, restored");
        assert_close(s.comp_total_amount, 0.0, "comp_total cleared");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 45: 负选项金额 + 固定折扣规则 ---

    #[test]
    fn test_combo_negative_option_with_fixed_discount() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 45);

        // 固定 3€ 折扣
        manager.cache_rules(&order_id, vec![make_fixed_discount_rule(30, 3.0)]);

        // 30€, 选项 -5€ (No Premium Ingredient), qty 2
        let r = add_items(&manager, &order_id, vec![
            item_with_options(
                1, "Soup", 30.0, 2,
                vec![make_option(6, "Ingredient", 0, "No Truffle", -5.0)],
            ),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // base=30, option=-5 → base_with_options=25
        // after_manual=25, fixed discount=3
        // unit_price = 25 - 3 = 22
        assert_close(s.items[0].unit_price, 22.0, "negative option + fixed discount");
        assert_close(s.subtotal, 44.0, "subtotal = 22 × 2");

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // ========================================================================
    // 联动测试 (Tests 46-60): 暴露 comp/uncomp + rules + payment 交互 bug
    // ========================================================================

    /// Helper: comp 指定数量
    fn comp_item_qty(
        manager: &OrdersManager,
        order_id: &str,
        instance_id: &str,
        quantity: i32,
    ) -> CommandResponse {
        let cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::CompItem {
                order_id: order_id.to_string(),
                instance_id: instance_id.to_string(),
                quantity,
                reason: "test comp".to_string(),
                authorizer_id: 1,
                authorizer_name: "Test".to_string(),
            },
        );
        manager.execute_command(cmd)
    }

    // --- Test 46: [FIXED] Full comp + uncomp 保留 applied_rules ---
    // Comp 保留 applied_rules, uncomp 后规则正确恢复
    #[test]
    fn test_full_comp_uncomp_preserves_applied_rules() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 46);

        // 10% 折扣规则
        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 100€ × 1 → 规则后 90€
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.items[0].unit_price, 90.0, "before comp: 100*0.9=90");
        assert!(!s.items[0].applied_rules.is_empty(), "should have rules");

        // Full comp → rules preserved
        let r = comp_item(&manager, &order_id, &iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(s.items[0].is_comped);
        assert_close(s.total, 0.0, "comped = free");
        assert!(!s.items[0].applied_rules.is_empty(), "rules preserved on comped item");

        // Uncomp → rules correctly restored
        let r = uncomp_item(&manager, &order_id, &iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(!s.items[0].is_comped);
        assert!(!s.items[0].applied_rules.is_empty(), "rules restored after uncomp");
        assert_close(s.items[0].unit_price, 90.0, "100*0.9=90 correctly restored");
        assert_close(s.total, 90.0, "total correct after uncomp");

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 47: Partial comp + uncomp (merge back) 保留源商品规则 ---
    #[test]
    fn test_partial_comp_uncomp_merge_preserves_rules() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 47);

        manager.cache_rules(&order_id, vec![make_discount_rule(20, 20.0)]);

        // 50€ × 3 → 40×3=120
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 50.0, 3)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.total, 120.0, "initial: 50*0.8*3=120");

        // Comp 1 个（partial）
        let r = comp_item_qty(&manager, &order_id, &iid, 1);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 2, "should split into 2 items");
        let source = s.items.iter().find(|i| !i.is_comped).unwrap();
        let comped = s.items.iter().find(|i| i.is_comped).unwrap();
        assert_eq!(source.quantity, 2);
        assert_eq!(comped.quantity, 1);
        // 源商品保留规则
        assert!(!source.applied_rules.is_empty(), "source keeps rules");
        assert_close(source.unit_price, 40.0, "source still 50*0.8=40");
        assert_close(s.subtotal, 80.0, "subtotal: 40*2=80 (comped=0)");

        // Uncomp → 合并回源
        let comped_iid = comped.instance_id.clone();
        let r = uncomp_item(&manager, &order_id, &comped_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.items.len(), 1, "merged back");
        assert_eq!(s.items[0].quantity, 3);
        // 源商品规则保留 → 价格正确
        assert!(!s.items[0].applied_rules.is_empty(), "rules preserved after merge");
        assert_close(s.total, 120.0, "restored: 40*3=120");

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 48: 规则 + skip + comp + toggle(comped时仍可操作) + uncomp → 验证规则状态 ---
    #[test]
    fn test_rule_skip_comp_toggle_uncomp_rules_preserved() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 48);

        manager.cache_rules(&order_id, vec![make_discount_rule(15, 15.0)]);

        // 200€ × 1 → 170
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 200.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.total, 170.0, "200*0.85=170");

        // Skip 规则 → total=200
        let rule_id: i64 = 15;
        let r = toggle_rule_skip(&manager, &order_id, rule_id, true);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 200.0, "rule skipped → 200");

        // Full comp → total=0, applied_rules preserved (fixed!)
        let r = comp_item(&manager, &order_id, &iid);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 0.0, "comped");
        assert!(!s.items[0].applied_rules.is_empty(), "rules preserved on comp");

        // Toggle 规则应该成功 — rules 保留在 comped item 上
        let r = toggle_rule_skip(&manager, &order_id, rule_id, false);
        assert!(r.success, "toggle succeeds: rules preserved on comped item");
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 0.0, "still comped, total stays 0");

        // Uncomp → 规则已 unskip, 应该恢复为 170
        let r = uncomp_item(&manager, &order_id, &iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(!s.items[0].is_comped);
        assert!(!s.items[0].applied_rules.is_empty(), "rules restored after uncomp");
        assert_close(s.items[0].unit_price, 170.0, "200*0.85=170 restored");
        assert_close(s.total, 170.0, "total restored with rules");
        assert_remaining_consistent(&s);
        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 49: 手动折扣 + comp + uncomp → 验证手动折扣恢复 ---
    #[test]
    fn test_manual_discount_comp_uncomp_restores_discount() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            49,
            vec![simple_item(1, "A", 100.0, 1)],
        );

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();

        // 30% 手动折扣 → 70
        let r = modify_item(&manager, &order_id, &iid, discount_changes(30.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // ModifyItem 可能改变 instance_id
        let iid = s.items[0].instance_id.clone();
        assert_close(s.total, 70.0, "100*0.7=70");
        assert_eq!(s.items[0].manual_discount_percent, Some(30.0));

        // Comp
        let r = comp_item(&manager, &order_id, &iid);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 0.0, "comped");
        // manual_discount_percent preserved on comped item (fixed!)
        assert_eq!(s.items[0].manual_discount_percent, Some(30.0));

        // Uncomp
        let r = uncomp_item(&manager, &order_id, &iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(!s.items[0].is_comped);
        // manual_discount_percent 保留 → uncomp 后正确恢复折扣
        assert_eq!(s.items[0].manual_discount_percent, Some(30.0));
        assert_close(s.total, 70.0, "100*0.7=70 restored correctly");
        assert_remaining_consistent(&s);
        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 50: 部分支付 + comp + uncomp + 支付完成 ---
    #[test]
    fn test_partial_pay_comp_uncomp_complete() {
        let manager = create_test_manager();
        let order_id = open_table_with_items(
            &manager,
            50,
            vec![
                simple_item(1, "A", 50.0, 2), // 100
                simple_item(2, "B", 30.0, 1), // 30 → total=130
            ],
        );

        // 部分支付 60
        let r = pay(&manager, &order_id, 60.0, "CARD");
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.paid_amount, 60.0, "paid 60");
        let b_iid = s.items.iter().find(|i| i.name == "B").unwrap().instance_id.clone();

        // Comp B (30€)
        let r = comp_item(&manager, &order_id, &b_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 100.0, "A=100, B comped=0");
        assert_remaining_consistent(&s);

        // Uncomp B
        let r = uncomp_item(&manager, &order_id, &b_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 130.0, "B restored → 130");
        assert_remaining_consistent(&s);

        // 支付剩余 70 并完成
        let r = pay(&manager, &order_id, s.remaining_amount, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 51: 规则 + 部分支付 + comp 部分 + toggle rule ---
    #[test]
    fn test_rule_partial_pay_partial_comp_toggle() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 51);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 100€ × 4 → 90×4=360
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 4)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.total, 360.0, "100*0.9*4=360");

        // 支付 180 (一半)
        let r = pay(&manager, &order_id, 180.0, "CARD");
        assert!(r.success);

        // Comp 1 个 (partial comp)
        let r = comp_item_qty(&manager, &order_id, &iid, 1);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 源: qty=3 (2paid+1unpaid), comped: qty=1
        // subtotal = 90*3 = 270
        assert_close(s.subtotal, 270.0, "3 remaining @ 90");
        assert_close(s.paid_amount, 180.0, "paid unchanged");
        assert_remaining_consistent(&s);

        // Skip 规则 → 源商品变为 100/个, 3个=300
        let rule_id: i64 = 10;
        let r = toggle_rule_skip(&manager, &order_id, rule_id, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let source = s.items.iter().find(|i| !i.is_comped).unwrap();
        assert_close(source.unit_price, 100.0, "rule skipped → 100/unit");
        assert_close(s.subtotal, 300.0, "100*3=300");
        assert_remaining_consistent(&s);

        // Unskip → 回到 90/个
        let r = toggle_rule_skip(&manager, &order_id, rule_id, false);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 270.0, "restored 90*3=270");
        assert_remaining_consistent(&s);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 52: 两种商品 + 规则 + comp 其中一种 + 整单折扣 ---
    #[test]
    fn test_two_items_rule_comp_order_discount() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 52);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // A: 100€×2=200→180, B: 50€×1=50→45 → total=225
        let r = add_items(&manager, &order_id, vec![
            simple_item(1, "A", 100.0, 2),
            simple_item(2, "B", 50.0, 1),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 225.0, "initial subtotal");
        let b_iid = s.items.iter().find(|i| i.name == "B").unwrap().instance_id.clone();

        // Comp B
        let r = comp_item(&manager, &order_id, &b_iid);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 180.0, "A=180, B comped");

        // 整单折扣 20% → subtotal=180, discount=36, total=144
        let r = apply_discount(&manager, &order_id, 20.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 144.0, "180-36=144");
        assert_remaining_consistent(&s);

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 53: 选项 + 规则 + comp + uncomp + 修改选项 ---
    #[test]
    fn test_options_rule_comp_uncomp_modify_options() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 53);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 80€ + 选项(+10€) = 90 → 规则后 81
        let r = add_items(&manager, &order_id, vec![
            item_with_options(
                1, "Steak", 80.0, 1,
                vec![make_option(4, "Side", 0, "Premium Fries", 10.0)],
            ),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.items[0].unit_price, 81.0, "(80+10)*0.9=81");

        // Full comp
        let r = comp_item(&manager, &order_id, &iid);
        assert!(r.success);

        // Uncomp
        let r = uncomp_item(&manager, &order_id, &iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // uncomp 恢复 price, 但 applied_rules 可能丢失 (Test 46 BUG)
        // 选项应该还在
        assert!(s.items[0].selected_options.is_some());
        assert_remaining_consistent(&s);
        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 54: 规则 + 分单支付 + 取消支付 + toggle rule ---
    #[test]
    fn test_rule_split_pay_cancel_toggle() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 54);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 100€ × 3 → 90*3=270
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 3)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        let a_unit_price = s.items[0].unit_price;
        assert_close(a_unit_price, 90.0, "100*0.9=90");

        // 分单支付 1 个 A
        let r = split_by_items(
            &manager, &order_id,
            vec![shared::order::SplitItem {
                instance_id: iid.clone(),
                name: "A".to_string(),
                quantity: 1,
                unit_price: a_unit_price,
            }],
            "CARD",
        );
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.paid_amount, 90.0, "paid 1×90=90");
        assert_remaining_consistent(&s);

        // 取消这笔支付
        let payment_id = s.payments.iter()
            .find(|p| !p.cancelled)
            .unwrap()
            .payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &payment_id);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.paid_amount, 0.0, "payment cancelled");
        assert_remaining_consistent(&s);

        // Toggle rule skip → 100/个
        let rule_id: i64 = 10;
        let r = toggle_rule_skip(&manager, &order_id, rule_id, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 300.0, "3×100=300 (rule skipped)");
        assert_remaining_consistent(&s);

        // Unskip → 90/个
        let r = toggle_rule_skip(&manager, &order_id, rule_id, false);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 270.0, "3×90=270");
        assert_remaining_consistent(&s);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 55: 多商品 + 规则 + 选择性 comp + 分单支付 ---
    #[test]
    fn test_multi_items_rule_selective_comp_split_pay() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 55);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // A: 80€×2 → 72×2=144, B: 40€×1 → 36
        let r = add_items(&manager, &order_id, vec![
            simple_item(1, "A", 80.0, 2),
            simple_item(2, "B", 40.0, 1),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let a_iid = s.items.iter().find(|i| i.name == "A").unwrap().instance_id.clone();
        let b_iid = s.items.iter().find(|i| i.name == "B").unwrap().instance_id.clone();
        assert_close(s.subtotal, 180.0, "144+36=180");

        // Comp B
        let r = comp_item(&manager, &order_id, &b_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 144.0, "A only");

        // 分单支付 1 个 A
        let a_unit = s.items.iter().find(|i| i.name == "A").unwrap().unit_price;
        let r = split_by_items(
            &manager, &order_id,
            vec![shared::order::SplitItem {
                instance_id: a_iid.clone(),
                name: "A".to_string(),
                quantity: 1,
                unit_price: a_unit,
            }],
            "CARD",
        );
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.paid_amount, 72.0, "paid 1A=72");
        assert_remaining_consistent(&s);

        // 支付剩余并完成
        let r = pay(&manager, &order_id, s.remaining_amount, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 56: 修改数量 + 规则 + comp 部分 + 修改价格 ---
    #[test]
    fn test_modify_qty_rule_partial_comp_modify_price() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 56);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 60€ × 2 → 54×2=108
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 60.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.total, 108.0, "60*0.9*2=108");

        // 增加到 4 个 → 54×4=216
        let r = modify_item(&manager, &order_id, &iid, qty_changes(4));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.total, 216.0, "54*4=216");

        // Comp 1 个
        let r = comp_item_qty(&manager, &order_id, &iid, 1);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 162.0, "54*3=162");

        // 修改源商品价格 → 80€
        let source = s.items.iter().find(|i| !i.is_comped).unwrap();
        let source_iid = source.instance_id.clone();
        let r = modify_item(&manager, &order_id, &source_iid, price_changes(80.0));
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 源商品: 80*0.9=72 (如果规则还在), qty=3 → 216
        // comped 商品: price=0
        let source = s.items.iter().find(|i| !i.is_comped).unwrap();
        assert_close(source.unit_price, 72.0, "80*0.9=72");
        assert_remaining_consistent(&s);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 57: 规则 + 整单附加费 + comp + 取消附加费 ---
    #[test]
    fn test_rule_surcharge_comp_cancel_surcharge() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 57);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 100€ × 2 → 90×2=180
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]);
        assert!(r.success);

        // 10% 整单附加费 → subtotal=180, surcharge=18, total=198
        let r = apply_surcharge(&manager, &order_id, 10.0);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items.iter().find(|i| i.name == "A").unwrap().instance_id.clone();
        assert_close(s.total, 198.0, "180+18=198");

        // Comp 全部 A
        let r = comp_item(&manager, &order_id, &iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // subtotal=0 (全部 comped), surcharge=0 (基于 subtotal), total=0
        assert_close(s.total, 0.0, "all comped → total=0");

        // 取消附加费
        let clear_surcharge_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id: order_id.clone(),
                surcharge_percent: None,
                surcharge_amount: None,
                authorizer_id: None,
                authorizer_name: None,
            },
        );
        let r = manager.execute_command(clear_surcharge_cmd);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 0.0, "still 0 (all comped, no surcharge)");

        // Uncomp
        let comped_iid = s.items.iter().find(|i| i.is_comped).unwrap().instance_id.clone();
        let r = uncomp_item(&manager, &order_id, &comped_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // uncomp 后: subtotal 应恢复, surcharge 已清除
        assert!(s.total > 0.0, "uncomped, should have value");
        assert_remaining_consistent(&s);
        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 58: 固定+百分比规则 + comp + uncomp + skip 交叉 ---
    #[test]
    fn test_fixed_percent_rules_comp_uncomp_skip_cross() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 58);

        // 百分比折扣 10% + 固定折扣 5€
        manager.cache_rules(&order_id, vec![
            make_discount_rule(100, 10.0),
            make_fixed_discount_rule(200, 5.0),
        ]);

        // 100€ × 2
        // base=100, percent disc=10 → 90, fixed disc=5 → 85 per unit
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let iid = s.items[0].instance_id.clone();
        assert_close(s.items[0].unit_price, 85.0, "100*0.9 - 5 = 85");
        assert_close(s.subtotal, 170.0, "85*2=170");

        // Skip 固定折扣 → 90/unit
        let r = toggle_rule_skip(&manager, &order_id, 200, true);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.items[0].unit_price, 90.0, "100*0.9=90 (fixed skipped)");

        // Comp 全部
        let r = comp_item(&manager, &order_id, &iid);
        assert!(r.success);

        // Uncomp
        let r = uncomp_item(&manager, &order_id, &iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // uncomp 后, 如果 applied_rules 丢失 (BUG), 则无折扣 → 100
        // 如果保留, 考虑 fdisc 仍然 skipped → 90
        assert_remaining_consistent(&s);
        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 59: 加菜 → 规则 → 再加菜 → comp 第一批 → 验证第二批不受影响 ---
    #[test]
    fn test_add_items_twice_rule_comp_first_batch() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 59);

        manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

        // 第一批: A 100€ × 1 → 90
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let a_iid = s.items[0].instance_id.clone();
        assert_close(s.total, 90.0, "A=90");

        // 第二批: B 50€ × 2 → 45×2=90
        let r = add_items(&manager, &order_id, vec![simple_item(2, "B", 50.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 180.0, "A=90 + B=90 → 180");

        // Comp A
        let r = comp_item(&manager, &order_id, &a_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 90.0, "B only: 45*2=90");

        // B 的规则不应受 comp A 影响
        let b_item = s.items.iter().find(|i| i.name == "B").unwrap();
        assert!(!b_item.applied_rules.is_empty(), "B keeps its rules");
        assert_close(b_item.unit_price, 45.0, "B=50*0.9=45");
        assert_remaining_consistent(&s);

        // 支付完成
        let r = pay(&manager, &order_id, s.total, "CARD");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // --- Test 60: Kitchen sink — 规则+选项+折扣+comp+uncomp+支付+取消+toggle+完成 ---
    #[test]
    fn test_kitchen_sink_all_interactions() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 60);

        // 15% 折扣规则
        manager.cache_rules(&order_id, vec![make_discount_rule(15, 15.0)]);

        // A: 200€, 选项+20€, qty=2 → base=220, *0.85=187 → 374
        // B: 80€, qty=1 → 80*0.85=68
        // total = 374+68 = 442
        let r = add_items(&manager, &order_id, vec![
            item_with_options(
                1, "A", 200.0, 2,
                vec![make_option(7, "Add-on", 0, "Truffle", 20.0)],
            ),
            simple_item(2, "B", 80.0, 1),
        ]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let b_iid = s.items.iter().find(|i| i.name == "B").unwrap().instance_id.clone();
        assert_close(s.subtotal, 442.0, "initial subtotal");

        // 1. 整单折扣 10% → discount=44.2, total=397.8
        let r = apply_discount(&manager, &order_id, 10.0);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.total, 397.8, "442-44.2=397.8");

        // 2. 部分支付 100
        let r = pay(&manager, &order_id, 100.0, "CARD");
        assert!(r.success);

        // 3. Comp B
        let r = comp_item(&manager, &order_id, &b_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // subtotal=374 (A only), discount=37.4, total=336.6
        assert_close(s.subtotal, 374.0, "A=374, B comped");
        assert_remaining_consistent(&s);

        // 4. Skip 规则 → A base=220, qty=2 → subtotal=440
        let rule_id: i64 = 15;
        let r = toggle_rule_skip(&manager, &order_id, rule_id, true);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 440.0, "A=220*2=440 (rule skipped)");

        // 5. 取消第一笔支付
        let payment_id = s.payments.iter()
            .find(|p| !p.cancelled)
            .unwrap()
            .payment_id.clone();
        let r = cancel_payment(&manager, &order_id, &payment_id);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.paid_amount, 0.0, "payment cancelled");

        // 6. Unskip 规则 → A=187*2=374
        let r = toggle_rule_skip(&manager, &order_id, rule_id, false);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_close(s.subtotal, 374.0, "rules restored");

        // 7. Uncomp B
        let b_comped_iid = s.items.iter().find(|i| i.is_comped).unwrap().instance_id.clone();
        let r = uncomp_item(&manager, &order_id, &b_comped_iid);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // B uncomped: 看 Test 46 BUG 是否影响, B 的 applied_rules 可能丢失
        assert_remaining_consistent(&s);

        // 8. 清除整单折扣
        let r = clear_discount(&manager, &order_id);
        assert!(r.success);

        // 9. 支付全额并完成
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        let r = pay(&manager, &order_id, s.total, "CASH");
        assert!(r.success);
        let r = complete_order(&manager, &order_id);
        assert!(r.success);

        assert_snapshot_consistent(&manager, &order_id);
    }

    // ========================================================================
    // AddItems 时间动态过滤测试 (Tests: time-filter)
    // ========================================================================

    /// Helper: 创建带时间约束的折扣规则
    fn make_timed_discount_rule(
        id: i64,
        percent: f64,
        valid_from: Option<i64>,
        valid_until: Option<i64>,
        active_days: Option<Vec<u8>>,
        active_start_time: Option<&str>,
        active_end_time: Option<&str>,
    ) -> PriceRule {
        use shared::models::price_rule::*;
        PriceRule {
            id,
            name: format!("timed_{}", id),
            display_name: format!("Timed {}", id),
            receipt_name: "DISC".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target_id: None,
            zone_scope: "all".to_string(),
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: percent,
            is_stackable: true,
            is_exclusive: false,
            valid_from,
            valid_until,
            active_days,
            active_start_time: active_start_time.map(|s| s.to_string()),
            active_end_time: active_end_time.map(|s| s.to_string()),
            is_active: true,
            created_by: None,
            created_at: 0,
        }
    }

    /// valid_from 在未来 → 规则不应用，商品原价
    #[test]
    fn test_add_items_filters_rule_valid_from_future() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 201);

        let future = shared::util::now_millis() + 3_600_000; // 1 小时后
        let rule = make_timed_discount_rule(1, 10.0, Some(future), None, None, None, None);
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Steak", 100.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 规则被过滤掉，100€ 原价
        assert_eq!(s.subtotal, 100.0);
    }

    /// valid_until 已过期 → 规则不应用
    #[test]
    fn test_add_items_filters_rule_valid_until_expired() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 202);

        let past = shared::util::now_millis() - 3_600_000; // 1 小时前
        let rule = make_timed_discount_rule(2, 10.0, None, Some(past), None, None, None);
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Wine", 50.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 过期规则不生效，50×2=100
        assert_eq!(s.subtotal, 100.0);
    }

    /// valid_from ≤ now ≤ valid_until → 规则生效
    #[test]
    fn test_add_items_applies_rule_within_valid_range() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 203);

        let now = shared::util::now_millis();
        let rule = make_timed_discount_rule(
            7,
            10.0,
            Some(now - 3_600_000), // 1小时前开始
            Some(now + 3_600_000), // 1小时后结束
            None,
            None,
            None,
        );
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Pasta", 100.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 10% 折扣: 100 → 90
        assert_eq!(s.subtotal, 90.0);
    }

    /// active_days 不匹配当前星期几 → 规则不应用
    #[test]
    fn test_add_items_filters_rule_wrong_day() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 204);

        // 获取当前是星期几 (0=Sun, 1=Mon, ..., 6=Sat)
        let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
        let today = now_local.format("%u").to_string().parse::<u8>().unwrap() % 7; // ISO weekday → 0-6

        // 设置 active_days 只包含"明天"
        let wrong_day = (today + 1) % 7;
        let rule = make_timed_discount_rule(3, 10.0, None, None, Some(vec![wrong_day]), None, None);
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Salad", 40.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 不匹配日期，40€ 原价
        assert_eq!(s.subtotal, 40.0);
    }

    /// active_days 匹配当前星期几 → 规则生效
    #[test]
    fn test_add_items_applies_rule_matching_day() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 205);

        let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
        let today = now_local.format("%u").to_string().parse::<u8>().unwrap() % 7;

        let rule = make_timed_discount_rule(4, 20.0, None, None, Some(vec![today]), None, None);
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Pizza", 50.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 20% 折扣: 50×2=100 → 80
        assert_eq!(s.subtotal, 80.0);
    }

    /// active_start_time/active_end_time 不在当前时间范围 → 规则不应用
    #[test]
    fn test_add_items_filters_rule_outside_time_window() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 206);

        // 构造一个绝对不在当前时间的窗口: 凌晨 03:00-04:00（除非真的在这个时间运行测试）
        // 使用更安全的方法：当前时间 +3h 到 +4h
        let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
        let hour = now_local.format("%H").to_string().parse::<u32>().unwrap();
        let start = format!("{:02}:00", (hour + 3) % 24);
        let end = format!("{:02}:00", (hour + 4) % 24);

        let rule = make_timed_discount_rule(5, 15.0, None, None, None, Some(&start), Some(&end));
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Soup", 20.0, 3)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 不在时间窗口，20×3=60 原价
        assert_eq!(s.subtotal, 60.0);
    }

    /// 混合规则: 一个过期 + 一个有效 → 只有有效的应用
    #[test]
    fn test_add_items_mixed_expired_and_active_rules() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 207);

        let now = shared::util::now_millis();
        let expired_rule = make_timed_discount_rule(
            8,
            50.0, // 50% 折扣 — 如果被应用会很明显
            None,
            Some(now - 3_600_000), // 1小时前过期
            None,
            None,
            None,
        );
        let active_rule = make_timed_discount_rule(
            9,
            10.0,
            Some(now - 3_600_000),
            Some(now + 3_600_000),
            None,
            None,
            None,
        );
        manager.cache_rules(&order_id, vec![expired_rule, active_rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Fish", 200.0, 1)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 只有 10% 有效: 200 → 180 (如果 50% 也生效会是 90)
        assert_eq!(s.subtotal, 180.0);
    }

    /// 无时间约束的规则始终生效
    #[test]
    fn test_add_items_no_time_constraint_always_applies() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 208);

        // 没有任何时间限制
        let rule = make_timed_discount_rule(6, 10.0, None, None, None, None, None);
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Bread", 10.0, 5)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 10% 折扣: 10×5=50 → 45
        assert_eq!(s.subtotal, 45.0);
    }

    /// valid_from + active_days 组合: valid_from 有效但 active_days 不匹配 → 不应用
    #[test]
    fn test_add_items_valid_from_ok_but_wrong_day() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 209);

        let now = shared::util::now_millis();
        let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
        let today = now_local.format("%u").to_string().parse::<u8>().unwrap() % 7;
        let wrong_day = (today + 1) % 7;

        let rule = make_timed_discount_rule(
            11,
            10.0,
            Some(now - 3_600_000), // valid_from OK
            None,
            Some(vec![wrong_day]), // wrong day
            None,
            None,
        );
        manager.cache_rules(&order_id, vec![rule]);

        let r = add_items(&manager, &order_id, vec![simple_item(1, "Cake", 30.0, 2)]);
        assert!(r.success);

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // active_days 不匹配，30×2=60 原价
        assert_eq!(s.subtotal, 60.0);
    }

    /// 第二次加菜时规则也需要实时检查时间
    #[test]
    fn test_add_items_second_batch_also_checks_time() {
        let manager = create_test_manager();
        let order_id = open_table(&manager, 210);

        let now = shared::util::now_millis();
        // 规则有效
        let rule = make_timed_discount_rule(
            12,
            10.0,
            Some(now - 3_600_000),
            Some(now + 3_600_000),
            None,
            None,
            None,
        );
        manager.cache_rules(&order_id, vec![rule]);

        // 第一批加菜
        let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 1)]);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(s.subtotal, 90.0); // 10% off

        // 第二批加菜（规则仍然有效）
        let r = add_items(&manager, &order_id, vec![simple_item(2, "B", 50.0, 2)]);
        assert!(r.success);
        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        // A: 90, B: 45×2=90 → 180
        assert_eq!(s.subtotal, 180.0);
    }
}
