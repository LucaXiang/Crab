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
use chrono::Local;
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
    pub fn new(db_path: impl AsRef<Path>) -> ManagerResult<Self> {
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
        })
    }

    /// Set the catalog service for product metadata lookup
    pub fn set_catalog_service(&mut self, catalog_service: Arc<crate::services::CatalogService>) {
        self.catalog_service = Some(catalog_service);
    }

    /// Set the archive service for SurrealDB integration
    pub fn set_archive_service(&mut self, db: surrealdb::Surreal<surrealdb::engine::local::Db>) {
        self.archive_service = Some(super::OrderArchiveService::new(db));
    }

    /// Generate next receipt number (crash-safe via redb)
    fn next_receipt_number(&self) -> String {
        let count = self.storage.next_order_count().unwrap_or(1);
        let date_str = Local::now().format("%Y%m%d").to_string();
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
        }
    }

    /// Get the server epoch (unique instance ID)
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// Cache rules for an order
    pub fn cache_rules(&self, order_id: &str, rules: Vec<PriceRule>) {
        let mut cache = self.rule_cache.write();
        cache.insert(order_id.to_string(), rules);
    }

    /// Get cached rules for an order
    pub fn get_cached_rules(&self, order_id: &str) -> Option<Vec<PriceRule>> {
        let cache = self.rule_cache.read();
        cache.get(order_id).cloned()
    }

    /// Remove cached rules for an order
    pub fn remove_cached_rules(&self, order_id: &str) {
        let mut cache = self.rule_cache.write();
        cache.remove(order_id);
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
                match self.storage.next_queue_number() {
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
                service_type,
            } => {
                tracing::info!(table_id = ?table_id, table_name = ?table_name, "Processing OpenTable command");
                // Use pre-generated receipt_number (generated before transaction)
                let receipt_number = pre_generated_receipt.expect("receipt_number must be pre-generated for OpenTable");
                CommandAction::OpenTable(super::actions::OpenTableAction {
                    table_id: table_id.clone(),
                    table_name: table_name.clone(),
                    zone_id: zone_id.clone(),
                    zone_name: zone_name.clone(),
                    guest_count: *guest_count,
                    is_retail: *is_retail,
                    service_type: *service_type,
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

        // 12. Clean up rule cache for completed/voided orders
        match &cmd.payload {
            shared::order::OrderCommandPayload::CompleteOrder { order_id, .. }
            | shared::order::OrderCommandPayload::VoidOrder { order_id, .. } => {
                self.remove_cached_rules(order_id);
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                service_type: None,
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
                    surcharge: None,
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
                    surcharge: None,
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
                service_type: None,
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
            surcharge: None,
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
    // 3. Merge 带支付的订单 — 支付信息正确转移
    // ========================================================================

    #[test]
    fn test_merge_orders_source_payments_are_transferred() {
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

        // Merge source → target
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

        let target_after = manager.get_snapshot(&target_id).unwrap().unwrap();
        // Items should be merged (source had 2 items [qty 2] + target had 1)
        assert!(target_after.items.len() > 1, "Target should have source items merged");

        // Source's payments should be transferred to target
        assert_eq!(
            target_after.paid_amount, 5.0,
            "Source order's payments should be transferred during merge"
        );
        assert_eq!(
            target_after.payments.len(), 1,
            "Source order's payment records should be transferred"
        );
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
                service_type: None,
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
                service_type: None,
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
                service_type: None,
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
}
