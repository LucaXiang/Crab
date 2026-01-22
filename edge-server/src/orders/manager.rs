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
use super::storage::{OrderStorage, StorageError};
use super::traits::{CommandContext, CommandHandler, CommandMetadata, EventApplier, OrderError};
use shared::order::{
    CommandError, CommandErrorCode, CommandResponse, OrderCommand, OrderEvent, OrderSnapshot,
    OrderStatus,
};
use std::path::Path;
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
        }
    }

    /// Get the server epoch (unique instance ID)
    pub fn epoch(&self) -> &str {
        &self.epoch
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
        let action: CommandAction = (&cmd).into();
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

        // 12. Return response
        let order_id = events.first().map(|e| e.order_id.clone());
        Ok((CommandResponse::success(cmd.command_id, order_id), events))
    }

    // ========== Public Query Methods ==========

    /// Get a snapshot by order ID
    pub fn get_snapshot(&self, order_id: &str) -> ManagerResult<Option<OrderSnapshot>> {
        Ok(self.storage.get_snapshot(order_id)?)
    }

    /// Get all active order snapshots
    pub fn get_active_orders(&self) -> ManagerResult<Vec<OrderSnapshot>> {
        Ok(self.storage.get_active_orders()?)
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
                    product_id: "prod-1".to_string(),
                    name: "Test Product".to_string(),
                    price: 10.0,
                    original_price: None,
                    quantity: 2,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
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
                    product_id: "prod-1".to_string(),
                    name: "Test Product".to_string(),
                    price: 10.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
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

    // ========== Event Chain Sequence Continuity Tests ==========

    #[test]
    fn test_sequence_continuity_single_order() {
        // Verify that events for a single order have strictly incrementing sequences
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
                    product_id: "prod-1".to_string(),
                    name: "Test Product".to_string(),
                    price: 10.0,
                    original_price: None,
                    quantity: 2,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        // Get events and verify sequence continuity
        let events = manager.storage.get_events_for_order(&order_id).unwrap();
        assert_eq!(events.len(), 2);

        // Verify sequences are strictly incrementing
        for i in 1..events.len() {
            assert!(
                events[i].sequence > events[i - 1].sequence,
                "Sequence must be strictly incrementing: {} should be > {}",
                events[i].sequence,
                events[i - 1].sequence
            );
        }

        // Note: Gaps are allowed in order sequences (unlike invoices)
        // We only verify sequences are unique and increasing
    }

    #[test]
    fn test_sequence_uniqueness_across_orders() {
        // Verify that global sequences are unique across all orders
        let manager = create_test_manager();

        // Create first order
        let cmd1 = create_open_table_cmd("op-1");
        let response1 = manager.execute_command(cmd1);
        let order_id1 = response1.order_id.unwrap();

        // Create second order
        let cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T2".to_string()),
                table_name: Some("Table 2".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 3,
                is_retail: false,
            },
        );
        let response2 = manager.execute_command(cmd2);
        let order_id2 = response2.order_id.unwrap();

        // Get all events
        let events1 = manager.storage.get_events_for_order(&order_id1).unwrap();
        let events2 = manager.storage.get_events_for_order(&order_id2).unwrap();

        // Collect all sequences
        let all_sequences: Vec<u64> = events1
            .iter()
            .chain(events2.iter())
            .map(|e| e.sequence)
            .collect();

        // Verify uniqueness
        let mut sorted = all_sequences.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            all_sequences.len(),
            sorted.len(),
            "All sequences must be unique across orders"
        );

        // Note: We verify uniqueness, not strict continuity
        // Gaps are allowed in order sequences
    }

    #[test]
    fn test_event_replay_reconstructs_snapshot() {
        // Verify that replaying events reconstructs the same snapshot
        use crate::orders::appliers::EventAction;
        use crate::orders::traits::EventApplier;

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
                    product_id: "prod-1".to_string(),
                    name: "Item A".to_string(),
                    price: 25.0,
                    original_price: None,
                    quantity: 3,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        // Get current snapshot (from storage)
        let stored_snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();

        // Get events and replay from scratch
        let events = manager.storage.get_events_for_order(&order_id).unwrap();

        // Replay events to rebuild snapshot
        let mut replayed_snapshot = shared::order::OrderSnapshot::new(order_id.clone());
        for event in &events {
            let action = EventAction::from(event);
            action.apply(&mut replayed_snapshot, event);
        }

        // Verify replayed snapshot matches stored snapshot
        assert_eq!(
            replayed_snapshot.order_id, stored_snapshot.order_id,
            "Order ID must match"
        );
        assert_eq!(
            replayed_snapshot.status, stored_snapshot.status,
            "Status must match"
        );
        assert_eq!(
            replayed_snapshot.items.len(),
            stored_snapshot.items.len(),
            "Item count must match"
        );
        assert_eq!(
            replayed_snapshot.subtotal, stored_snapshot.subtotal,
            "Subtotal must match"
        );
        assert_eq!(
            replayed_snapshot.total, stored_snapshot.total,
            "Total must match"
        );
        assert_eq!(
            replayed_snapshot.last_sequence, stored_snapshot.last_sequence,
            "Last sequence must match"
        );

        // Verify checksum matches
        assert_eq!(
            replayed_snapshot.state_checksum, stored_snapshot.state_checksum,
            "Checksum must match after replay"
        );
    }

    #[test]
    fn test_sequential_command_sequence_integrity() {
        // In-memory test: sequential commands produce sequential events
        // Note: In production, gaps may occur due to failures/rollbacks
        let manager = create_test_manager();

        // Execute 10 commands
        for i in 0..10 {
            let cmd = OrderCommand::new(
                "op-1".to_string(),
                "Test Operator".to_string(),
                OrderCommandPayload::OpenTable {
                    table_id: Some(format!("T{}", i)),
                    table_name: Some(format!("Table {}", i)),
                    zone_id: None,
                    zone_name: None,
                    guest_count: 2,
                    is_retail: false,
                },
            );
            manager.execute_command(cmd);
        }

        // Get all events
        let events = manager.storage.get_events_since(0).unwrap();
        assert_eq!(events.len(), 10);

        // Verify strict sequence order
        for (i, event) in events.iter().enumerate() {
            assert_eq!(
                event.sequence,
                (i + 1) as u64,
                "Event {} should have sequence {}",
                i,
                i + 1
            );
        }
    }

    #[test]
    fn test_duplicate_command_rejected_cleanly() {
        // Verify that duplicate commands are rejected without side effects
        let manager = create_test_manager();

        // First command
        let cmd = create_open_table_cmd("op-1");
        let response1 = manager.execute_command(cmd.clone());
        assert!(response1.order_id.is_some());

        // Duplicate command (same command_id)
        let response2 = manager.execute_command(cmd);
        assert!(response2.order_id.is_none()); // Duplicate

        // New command
        let cmd2 = OrderCommand::new(
            "op-1".to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T2".to_string()),
                table_name: Some("Table 2".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
            },
        );
        let response3 = manager.execute_command(cmd2);
        assert!(response3.order_id.is_some());

        // Verify only 2 events created (duplicate rejected)
        let events = manager.storage.get_events_since(0).unwrap();
        assert_eq!(events.len(), 2, "Duplicate should not create event");
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

    // ========== Tampering Detection Tests ==========

    #[test]
    fn test_tampering_total_detected() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        // Add item
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "prod-1".to_string(),
                    name: "Item".to_string(),
                    price: 100.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        let mut snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.verify_checksum(), "Original checksum valid");

        // Tamper with total
        snapshot.total = 50.0;
        assert!(!snapshot.verify_checksum(), "Tampering detected");
    }

    #[test]
    fn test_tampering_add_item_detected() {
        // Checksum includes items.len(), so adding items is detected
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "prod-1".to_string(),
                    name: "Item".to_string(),
                    price: 100.0,
                    original_price: None,
                    quantity: 2,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        let mut snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.verify_checksum());

        // Tamper by adding a fake item (changes items.len())
        snapshot.items.push(snapshot.items[0].clone());
        assert!(!snapshot.verify_checksum(), "Item count tampering detected");
    }

    #[test]
    fn test_tampering_remove_item_detected() {
        // Checksum includes items.len(), so removing items is detected
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![
                    CartItemInput {
                        product_id: "prod-1".to_string(),
                        name: "Item 1".to_string(),
                        price: 10.0,
                        original_price: None,
                        quantity: 1,
                        selected_options: None,
                        selected_specification: None,
                        discount_percent: None,
                        surcharge: None,
                        note: None,
                        authorizer_id: None,
                        authorizer_name: None,
                    },
                    CartItemInput {
                        product_id: "prod-2".to_string(),
                        name: "Item 2".to_string(),
                        price: 20.0,
                        original_price: None,
                        quantity: 1,
                        selected_options: None,
                        selected_specification: None,
                        discount_percent: None,
                        surcharge: None,
                        note: None,
                        authorizer_id: None,
                        authorizer_name: None,
                    },
                ],
            },
        );
        manager.execute_command(add_cmd);

        let mut snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items.len(), 2);
        assert!(snapshot.verify_checksum());

        // Tamper by removing an item (changes items.len())
        snapshot.items.pop();
        assert!(!snapshot.verify_checksum(), "Item removal tampering detected");
    }

    #[test]
    fn test_tampering_status_detected() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        let mut snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.status, OrderStatus::Active);
        assert!(snapshot.verify_checksum());

        // Tamper with status
        snapshot.status = OrderStatus::Completed;
        assert!(!snapshot.verify_checksum(), "Status tampering detected");
    }

    #[test]
    fn test_tampering_sequence_detected() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        let mut snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(snapshot.verify_checksum());

        // Tamper with sequence
        snapshot.last_sequence = 999;
        assert!(!snapshot.verify_checksum(), "Sequence tampering detected");
    }

    // ========== Boundary Condition Tests ==========

    #[test]
    fn test_large_amount_precision() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        // Add item with large price
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "prod-1".to_string(),
                    name: "Expensive Item".to_string(),
                    price: 999999.99,
                    original_price: None,
                    quantity: 100,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 999999.99 * 100 = 99999999.00
        assert_eq!(snapshot.total, 99999999.0);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_small_amount_precision() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        // Add many small items
        for i in 0..100 {
            let add_cmd = OrderCommand::new(
                "op-1".to_string(),
                "Test".to_string(),
                OrderCommandPayload::AddItems {
                    order_id: order_id.clone(),
                    items: vec![CartItemInput {
                        product_id: format!("prod-{}", i),
                        name: "Penny Item".to_string(),
                        price: 0.01,
                        original_price: None,
                        quantity: 1,
                        selected_options: None,
                        selected_specification: None,
                        discount_percent: None,
                        surcharge: None,
                        note: None,
                        authorizer_id: None,
                        authorizer_name: None,
                    }],
                },
            );
            manager.execute_command(add_cmd);
        }

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        // 0.01 * 100 = 1.00 (must be exact, not 0.9999... or 1.0001...)
        assert_eq!(snapshot.total, 1.0);
    }

    #[test]
    fn test_zero_price_item() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "prod-free".to_string(),
                    name: "Free Item".to_string(),
                    price: 0.0,
                    original_price: None,
                    quantity: 5,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: None,
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 0.0);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_100_percent_discount() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "prod-1".to_string(),
                    name: "Full Discount Item".to_string(),
                    price: 100.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: Some(100.0),
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 0.0);
    }

    #[test]
    fn test_fractional_discount_precision() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        // 33.33% discount on $100 = $66.67
        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: "prod-1".to_string(),
                    name: "Item".to_string(),
                    price: 100.0,
                    original_price: None,
                    quantity: 1,
                    selected_options: None,
                    selected_specification: None,
                    discount_percent: Some(33.33),
                    surcharge: None,
                    note: None,
                    authorizer_id: None,
                    authorizer_name: None,
                }],
            },
        );
        manager.execute_command(add_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.total, 66.67);
    }

    #[test]
    fn test_many_items_order() {
        let manager = create_test_manager();

        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        // Add 50 different items
        let items: Vec<CartItemInput> = (0..50)
            .map(|i| CartItemInput {
                product_id: format!("prod-{}", i),
                name: format!("Item {}", i),
                price: 10.0 + (i as f64 * 0.1),
                original_price: None,
                quantity: 1,
                selected_options: None,
                selected_specification: None,
                discount_percent: None,
                surcharge: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            })
            .collect();

        let add_cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items,
            },
        );
        manager.execute_command(add_cmd);

        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items.len(), 50);
        assert!(snapshot.verify_checksum());
    }

    // ========== Concurrency Tests ==========

    #[test]
    fn test_concurrent_order_creation() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(create_test_manager());
        let mut handles = vec![];

        // Spawn 10 threads, each creating an order
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let cmd = OrderCommand::new(
                    format!("op-{}", i),
                    "Test".to_string(),
                    OrderCommandPayload::OpenTable {
                        table_id: Some(format!("T{}", i)),
                        table_name: Some(format!("Table {}", i)),
                        zone_id: None,
                        zone_name: None,
                        guest_count: 2,
                        is_retail: false,
                    },
                );
                manager_clone.execute_command(cmd)
            });
            handles.push(handle);
        }

        // Wait for all threads
        let responses: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All should succeed
        assert!(responses.iter().all(|r| r.success));

        // All order IDs should be unique
        let order_ids: Vec<_> = responses.iter().filter_map(|r| r.order_id.clone()).collect();
        assert_eq!(order_ids.len(), 10);

        let mut unique_ids = order_ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(unique_ids.len(), 10, "All order IDs must be unique");

        // All sequences should be unique
        let events = manager.storage.get_events_since(0).unwrap();
        let sequences: Vec<_> = events.iter().map(|e| e.sequence).collect();
        let mut unique_seqs = sequences.clone();
        unique_seqs.sort();
        unique_seqs.dedup();
        assert_eq!(unique_seqs.len(), 10, "All sequences must be unique");
    }

    #[test]
    fn test_concurrent_add_items_same_order() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(create_test_manager());

        // Create order first
        let cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
            },
        );
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        let mut handles = vec![];

        // Spawn 5 threads, each adding items to same order
        for i in 0..5 {
            let manager_clone = Arc::clone(&manager);
            let order_id_clone = order_id.clone();
            let handle = thread::spawn(move || {
                let add_cmd = OrderCommand::new(
                    format!("op-add-{}", i),
                    "Test".to_string(),
                    OrderCommandPayload::AddItems {
                        order_id: order_id_clone,
                        items: vec![CartItemInput {
                            product_id: format!("prod-{}", i),
                            name: format!("Item {}", i),
                            price: 10.0,
                            original_price: None,
                            quantity: 1,
                            selected_options: None,
                            selected_specification: None,
                            discount_percent: None,
                            surcharge: None,
                            note: None,
                            authorizer_id: None,
                            authorizer_name: None,
                        }],
                    },
                );
                manager_clone.execute_command(add_cmd)
            });
            handles.push(handle);
        }

        // Wait for all threads
        let responses: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All should succeed
        assert!(responses.iter().all(|r| r.success));

        // Verify final state
        let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert_eq!(snapshot.items.len(), 5);
        assert_eq!(snapshot.total, 50.0); // 5 items * $10
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_concurrent_idempotency() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(create_test_manager());

        // Create a command with fixed ID
        let mut cmd = OrderCommand::new(
            "op-1".to_string(),
            "Test".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
            },
        );
        cmd.command_id = "fixed-cmd-id".to_string();

        let mut handles = vec![];

        // Spawn 10 threads, all trying to execute same command
        for _ in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let cmd_clone = cmd.clone();
            let handle = thread::spawn(move || manager_clone.execute_command(cmd_clone));
            handles.push(handle);
        }

        // Wait for all threads
        let responses: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // Exactly one should create the order
        let created_count = responses.iter().filter(|r| r.order_id.is_some()).count();
        assert_eq!(created_count, 1, "Exactly one thread should create the order");

        // Only one event should exist
        let events = manager.storage.get_events_since(0).unwrap();
        assert_eq!(events.len(), 1, "Only one event should be created");
    }
}
