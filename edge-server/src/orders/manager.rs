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
use shared::order::{
    CommandError, CommandErrorCode, CommandResponse, OrderCommand, OrderEvent, OrderSnapshot,
    OrderStatus,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
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

impl From<ManagerError> for CommandError {
    fn from(err: ManagerError) -> Self {
        let (code, message) = match err {
            ManagerError::Storage(e) => (CommandErrorCode::InternalError, e.to_string()),
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

/// Event broadcast channel capacity
const EVENT_CHANNEL_CAPACITY: usize = 1024;

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
    /// Cached product metadata (category_id, tags) for rule matching
    product_meta_cache: Arc<RwLock<HashMap<String, ProductMeta>>>,
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
            product_meta_cache: Arc::new(RwLock::new(HashMap::new())),
        })
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
            product_meta_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the server epoch (unique instance ID)
    pub fn epoch(&self) -> &str {
        &self.epoch
    }

    /// Cache rules for an order
    pub fn cache_rules(&self, order_id: &str, rules: Vec<PriceRule>) {
        let mut cache = self.rule_cache.write().unwrap();
        cache.insert(order_id.to_string(), rules);
    }

    /// Get cached rules for an order
    pub fn get_cached_rules(&self, order_id: &str) -> Option<Vec<PriceRule>> {
        let cache = self.rule_cache.read().unwrap();
        cache.get(order_id).cloned()
    }

    /// Remove cached rules for an order
    pub fn remove_cached_rules(&self, order_id: &str) {
        let mut cache = self.rule_cache.write().unwrap();
        cache.remove(order_id);
    }

    /// Cache product metadata for a product
    pub fn cache_product_meta(&self, product_id: &str, meta: ProductMeta) {
        let mut cache = self.product_meta_cache.write().unwrap();
        cache.insert(product_id.to_string(), meta);
    }

    /// Batch cache product metadata
    pub fn cache_product_metadata_batch(&self, metadata: HashMap<String, ProductMeta>) {
        let mut cache = self.product_meta_cache.write().unwrap();
        cache.extend(metadata);
    }

    /// Get cached product metadata for a product
    pub fn get_product_meta(&self, product_id: &str) -> Option<ProductMeta> {
        let cache = self.product_meta_cache.read().unwrap();
        cache.get(product_id).cloned()
    }

    /// Get product metadata for a list of items
    fn get_product_metadata_for_items(
        &self,
        items: &[shared::order::CartItemInput],
    ) -> HashMap<String, ProductMeta> {
        let cache = self.product_meta_cache.read().unwrap();
        items
            .iter()
            .filter_map(|item| {
                cache
                    .get(&item.product_id)
                    .map(|meta| (item.product_id.clone(), meta.clone()))
            })
            .collect()
    }

    /// Clear product metadata cache
    pub fn clear_product_meta_cache(&self) {
        let mut cache = self.product_meta_cache.write().unwrap();
        cache.clear();
    }

    /// Subscribe to event broadcasts
    pub fn subscribe(&self) -> broadcast::Receiver<OrderEvent> {
        self.event_tx.subscribe()
    }

    /// Get the underlying storage
    pub fn storage(&self) -> &OrderStorage {
        &self.storage
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
        // 1. Idempotency check
        if self.storage.is_command_processed(&cmd.command_id)? {
            return Ok((CommandResponse::duplicate(cmd.command_id), vec![]));
        }

        // 2. Begin write transaction
        let txn = self.storage.begin_write()?;

        // Double-check idempotency within transaction
        if self
            .storage
            .is_command_processed_txn(&txn, &cmd.command_id)?
        {
            return Ok((CommandResponse::duplicate(cmd.command_id), vec![]));
        }

        // 3. Get current sequence for context initialization
        let current_sequence = self.storage.get_current_sequence()?;

        // 4. Create context and metadata
        let mut ctx = CommandContext::new(&txn, &self.storage, current_sequence);
        let metadata = CommandMetadata {
            command_id: cmd.command_id.clone(),
            operator_id: cmd.operator_id.clone(),
            operator_name: cmd.operator_name.clone(),
            timestamp: cmd.timestamp,
        };

        // 5. Convert to action and execute (blocking async)
        // For AddItems commands, inject cached price rules and product metadata
        let action: CommandAction = match &cmd.payload {
            shared::order::OrderCommandPayload::AddItems { order_id, items } => {
                let rules = self.get_cached_rules(order_id).unwrap_or_default();
                // Look up product metadata for each item from cache
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
        let order_id = events.first().map(|e| e.order_id.clone());
        Ok((CommandResponse::success(cmd.command_id, order_id), events))
    }

    // ========== Public Query Methods ==========

    /// Get a snapshot by order ID
    pub fn get_snapshot(&self, order_id: &str) -> ManagerResult<Option<OrderSnapshot>> {
        let mut snapshot = self.storage.get_snapshot(order_id)?;
        // Ensure line_total is populated for backward compatibility with old data
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
        // Ensure line_total is populated for backward compatibility with old data
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
            product_meta_cache: self.product_meta_cache.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{CartItemInput, OrderCommandPayload, OrderEventType, PaymentInput};

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
                    method: "cash".to_string(),
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

        // Complete order
        let complete_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::CompleteOrder {
                order_id: order_id.clone(),
                receipt_number: "R001".to_string(),
            },
        );
        let complete_response = manager.execute_command(complete_cmd);
        assert!(complete_response.success);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Completed);
        assert_eq!(snapshot.receipt_number, Some("R001".to_string()));
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
                reason: Some("Customer cancelled".to_string()),
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
}
