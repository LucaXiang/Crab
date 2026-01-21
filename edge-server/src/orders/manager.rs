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
//!     ├─ 3. Process command → generate event(s)
//!     ├─ 4. Persist event(s)
//!     ├─ 5. Update snapshot
//!     ├─ 6. Mark command processed
//!     ├─ 7. Commit transaction
//!     ├─ 8. Broadcast event(s)
//!     └─ 9. Return response
//! ```

use super::reducer::{OrderReducer, generate_instance_id, input_to_snapshot};
use super::storage::{OrderStorage, StorageError};
use shared::order::{
    CartItemInput, CartItemSnapshot, CommandError, CommandErrorCode, CommandResponse, EventPayload,
    ItemChanges, ItemModificationResult, OrderCommand, OrderCommandPayload, OrderEvent,
    OrderEventType, OrderSnapshot, OrderStatus, PaymentSummaryItem, SplitItem,
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

        // 3. Process command based on payload
        let result = match &cmd.payload {
            OrderCommandPayload::OpenTable { .. } => self.handle_open_table(&txn, &cmd),
            OrderCommandPayload::CompleteOrder {
                order_id,
                receipt_number,
            } => self.handle_complete_order(&txn, &cmd, order_id, receipt_number),
            OrderCommandPayload::VoidOrder { order_id, reason } => {
                self.handle_void_order(&txn, &cmd, order_id, reason.clone())
            }
            OrderCommandPayload::RestoreOrder { order_id } => {
                self.handle_restore_order(&txn, &cmd, order_id)
            }
            OrderCommandPayload::AddItems { order_id, items } => {
                self.handle_add_items(&txn, &cmd, order_id, items)
            }
            OrderCommandPayload::ModifyItem {
                order_id,
                instance_id,
                affected_quantity,
                changes,
                authorizer_id,
                authorizer_name,
            } => self.handle_modify_item(
                &txn,
                &cmd,
                order_id,
                instance_id,
                *affected_quantity,
                changes,
                authorizer_id.clone(),
                authorizer_name.clone(),
            ),
            OrderCommandPayload::RemoveItem {
                order_id,
                instance_id,
                quantity,
                reason,
                authorizer_id,
                authorizer_name,
            } => self.handle_remove_item(
                &txn,
                &cmd,
                order_id,
                instance_id,
                *quantity,
                reason.clone(),
                authorizer_id.clone(),
                authorizer_name.clone(),
            ),
            OrderCommandPayload::RestoreItem {
                order_id,
                instance_id,
            } => self.handle_restore_item(&txn, &cmd, order_id, instance_id),
            OrderCommandPayload::AddPayment { order_id, payment } => {
                self.handle_add_payment(&txn, &cmd, order_id, payment)
            }
            OrderCommandPayload::CancelPayment {
                order_id,
                payment_id,
                reason,
                authorizer_id,
                authorizer_name,
            } => self.handle_cancel_payment(
                &txn,
                &cmd,
                order_id,
                payment_id,
                reason.clone(),
                authorizer_id.clone(),
                authorizer_name.clone(),
            ),
            OrderCommandPayload::SplitOrder {
                order_id,
                split_amount,
                payment_method,
                items,
            } => {
                self.handle_split_order(&txn, &cmd, order_id, *split_amount, payment_method, items)
            }
            OrderCommandPayload::MoveOrder {
                order_id,
                target_table_id,
                target_table_name,
                target_zone_name,
            } => self.handle_move_order(
                &txn,
                &cmd,
                order_id,
                target_table_id,
                target_table_name,
                target_zone_name.clone(),
            ),
            OrderCommandPayload::MergeOrders {
                source_order_id,
                target_order_id,
            } => self.handle_merge_orders(&txn, &cmd, source_order_id, target_order_id),
            OrderCommandPayload::UpdateOrderInfo {
                order_id,
                receipt_number,
                guest_count,
                table_name,
                is_pre_payment,
            } => self.handle_update_order_info(
                &txn,
                &cmd,
                order_id,
                receipt_number.clone(),
                *guest_count,
                table_name.clone(),
                *is_pre_payment,
            ),
        };

        match result {
            Ok((response, events)) => {
                // 6. Mark command processed
                self.storage.mark_command_processed(&txn, &cmd.command_id)?;

                // 7. Commit transaction
                txn.commit().map_err(StorageError::from)?;

                Ok((response, events))
            }
            Err(e) => {
                // Transaction will be rolled back on drop
                Err(e)
            }
        }
    }

    // ========== Command Handlers ==========

    fn handle_open_table(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let (table_id, table_name, zone_id, zone_name, guest_count, is_retail) = match &cmd.payload
        {
            OrderCommandPayload::OpenTable {
                table_id,
                table_name,
                zone_id,
                zone_name,
                guest_count,
                is_retail,
            } => (
                table_id.clone(),
                table_name.clone(),
                zone_id.clone(),
                zone_name.clone(),
                *guest_count,
                *is_retail,
            ),
            _ => unreachable!(),
        };

        // Generate new order ID
        let order_id = uuid::Uuid::new_v4().to_string();
        let sequence = self.storage.increment_sequence(txn)?;

        // Create event
        let event = OrderEvent::from_command(
            sequence,
            order_id.clone(),
            cmd,
            OrderEventType::TableOpened,
            EventPayload::TableOpened {
                table_id,
                table_name,
                zone_id,
                zone_name,
                guest_count,
                is_retail,
                receipt_number: None,
            },
        );

        // Store event
        self.storage.store_event(txn, &event)?;

        // Create and store snapshot
        let snapshot = OrderReducer::create_snapshot(&event)
            .ok_or_else(|| ManagerError::Internal("Failed to create snapshot".to_string()))?;
        self.storage.store_snapshot(txn, &snapshot)?;

        // Mark order as active
        self.storage.mark_order_active(txn, &order_id)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), Some(order_id)),
            vec![event],
        ))
    }

    fn handle_complete_order(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        receipt_number: &str,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;

        // Calculate payment summary and total paid
        let mut payment_summary: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let mut total_paid = 0.0_f64;
        for payment in &snapshot.payments {
            if !payment.cancelled {
                *payment_summary.entry(payment.method.clone()).or_insert(0.0) += payment.amount;
                total_paid += payment.amount;
            }
        }

        // Validate payment is sufficient (allow 0.01 tolerance for rounding)
        if total_paid < snapshot.total - 0.01 {
            return Err(ManagerError::InvalidOperation(format!(
                "Payment insufficient: paid {:.2}, required {:.2}",
                total_paid, snapshot.total
            )));
        }

        let payment_summary: Vec<PaymentSummaryItem> = payment_summary
            .into_iter()
            .map(|(method, amount)| PaymentSummaryItem { method, amount })
            .collect();

        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::OrderCompleted,
            EventPayload::OrderCompleted {
                receipt_number: receipt_number.to_string(),
                final_total: snapshot.total,
                payment_summary,
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        // Mark order as inactive
        self.storage.mark_order_inactive(txn, order_id)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    fn handle_void_order(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        reason: Option<String>,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;
        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::OrderVoided,
            EventPayload::OrderVoided {
                reason,
                authorizer_id: None,
                authorizer_name: None,
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        // Mark order as inactive
        self.storage.mark_order_inactive(txn, order_id)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    fn handle_restore_order(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self
            .storage
            .get_snapshot_txn(txn, order_id)?
            .ok_or_else(|| ManagerError::OrderNotFound(order_id.to_string()))?;

        if snapshot.status != OrderStatus::Void {
            return Err(ManagerError::Internal(
                "Can only restore voided orders".to_string(),
            ));
        }

        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::OrderRestored,
            EventPayload::OrderRestored {},
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        // Mark order as active again
        self.storage.mark_order_active(txn, order_id)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    fn handle_add_items(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        items: &[CartItemInput],
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;
        let sequence = self.storage.increment_sequence(txn)?;

        // Convert inputs to snapshots with generated instance_ids
        let item_snapshots: Vec<CartItemSnapshot> = items.iter().map(input_to_snapshot).collect();

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::ItemsAdded,
            EventPayload::ItemsAdded {
                items: item_snapshots,
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_modify_item(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        instance_id: &str,
        affected_quantity: Option<i32>,
        changes: &ItemChanges,
        authorizer_id: Option<String>,
        authorizer_name: Option<String>,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;

        // Find the item
        let item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == instance_id)
            .ok_or_else(|| ManagerError::ItemNotFound(instance_id.to_string()))?;

        let affected_qty = affected_quantity.unwrap_or(item.quantity);
        if affected_qty > item.quantity {
            return Err(ManagerError::InsufficientQuantity);
        }

        // Calculate previous values for audit
        let previous_values = ItemChanges {
            price: if changes.price.is_some() {
                Some(item.price)
            } else {
                None
            },
            quantity: if changes.quantity.is_some() {
                Some(item.quantity)
            } else {
                None
            },
            discount_percent: if changes.discount_percent.is_some() {
                item.discount_percent
            } else {
                None
            },
            surcharge: if changes.surcharge.is_some() {
                item.surcharge
            } else {
                None
            },
            note: if changes.note.is_some() {
                item.note.clone()
            } else {
                None
            },
        };

        // Determine operation type
        let operation = if changes.discount_percent.is_some() {
            "APPLY_DISCOUNT"
        } else if changes.price.is_some() {
            "MODIFY_PRICE"
        } else if changes.quantity.is_some() {
            "MODIFY_QUANTITY"
        } else {
            "MODIFY_ITEM"
        };

        // Calculate results
        let results = if affected_qty >= item.quantity {
            // Modifying entire item
            vec![ItemModificationResult {
                instance_id: instance_id.to_string(),
                quantity: item.quantity,
                price: changes.price.unwrap_or(item.price),
                discount_percent: changes.discount_percent.or(item.discount_percent),
                action: "UPDATED".to_string(),
            }]
        } else {
            // Split: some unchanged, some modified
            let new_price = changes.price.unwrap_or(item.price);
            let new_discount = changes.discount_percent.or(item.discount_percent);
            let new_instance_id = generate_instance_id(
                &item.id,
                new_price,
                new_discount,
                &item.selected_options,
                &item.selected_specification,
                changes.surcharge.or(item.surcharge),
            );

            vec![
                ItemModificationResult {
                    instance_id: instance_id.to_string(),
                    quantity: item.quantity - affected_qty,
                    price: item.price,
                    discount_percent: item.discount_percent,
                    action: "UNCHANGED".to_string(),
                },Box::new()
                ItemModificationResult {
                    instance_id: new_instance_id,
                    quantity: affected_qty,
                    price: new_price,
                    discount_percent: new_discount,
                    action: "CREATED".to_string(),
                },
            ]
        };

        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::ItemModified,
            EventPayload::ItemModified {
                operation: operation.to_string(),
                source: Box::new(item.clone()),
                affected_quantity: affected_qty,
                changes: changes.clone(),
                previous_values,
                results,
                authorizer_id,
                authorizer_name,
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_remove_item(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        instance_id: &str,
        quantity: Option<i32>,
        reason: Option<String>,
        authorizer_id: Option<String>,
        authorizer_name: Option<String>,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;

        // Find the item
        let item = snapshot
            .items
            .iter()
            .find(|i| i.instance_id == instance_id)
            .ok_or_else(|| ManagerError::ItemNotFound(instance_id.to_string()))?;

        if let Some(qty) = quantity
            && qty > item.quantity
        {
            return Err(ManagerError::InsufficientQuantity);
        }

        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::ItemRemoved,
            EventPayload::ItemRemoved {
                instance_id: instance_id.to_string(),
                item_name: item.name.clone(),
                quantity,
                reason,
                authorizer_id,
                authorizer_name,
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    fn handle_restore_item(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        instance_id: &str,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;
        let sequence = self.storage.increment_sequence(txn)?;

        // Note: Full implementation would require tracking removed items
        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::ItemRestored,
            EventPayload::ItemRestored {
                instance_id: instance_id.to_string(),
                item_name: "Unknown".to_string(), // Would need to track removed items
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    fn handle_add_payment(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        payment: &shared::order::PaymentInput,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;

        if payment.amount <= 0.0 {
            return Err(ManagerError::InvalidAmount);
        }

        let payment_id = uuid::Uuid::new_v4().to_string();
        let sequence = self.storage.increment_sequence(txn)?;

        // Calculate change for cash payments
        let change = payment.tendered.map(|t| (t - payment.amount).max(0.0));

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::PaymentAdded,
            EventPayload::PaymentAdded {
                payment_id,
                method: payment.method.clone(),
                amount: payment.amount,
                tendered: payment.tendered,
                change,
                note: payment.note.clone(),
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_cancel_payment(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        payment_id: &str,
        reason: Option<String>,
        authorizer_id: Option<String>,
        authorizer_name: Option<String>,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;

        // Find the payment
        let payment = snapshot
            .payments
            .iter()
            .find(|p| p.payment_id == payment_id && !p.cancelled)
            .ok_or_else(|| ManagerError::PaymentNotFound(payment_id.to_string()))?;

        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::PaymentCancelled,
            EventPayload::PaymentCancelled {
                payment_id: payment_id.to_string(),
                method: payment.method.clone(),
                amount: payment.amount,
                reason,
                authorizer_id,
                authorizer_name,
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    fn handle_split_order(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        split_amount: f64,
        payment_method: &str,
        items: &[SplitItem],
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;

        if split_amount <= 0.0 {
            return Err(ManagerError::InvalidAmount);
        }

        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::OrderSplit,
            EventPayload::OrderSplit {
                split_amount,
                payment_method: payment_method.to_string(),
                items: items.to_vec(),
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_move_order(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        target_table_id: &str,
        target_table_name: &str,
        _target_zone_name: Option<String>,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;
        let sequence = self.storage.increment_sequence(txn)?;

        let source_table_id = snapshot.table_id.clone().unwrap_or_default();
        let source_table_name = snapshot.table_name.clone().unwrap_or_default();

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::OrderMoved,
            EventPayload::OrderMoved {
                source_table_id,
                source_table_name,
                target_table_id: target_table_id.to_string(),
                target_table_name: target_table_name.to_string(),
                items: snapshot.items.clone(),
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    fn handle_merge_orders(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        source_order_id: &str,
        target_order_id: &str,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let source_snapshot = self.get_active_snapshot(txn, source_order_id)?;
        let target_snapshot = self.get_active_snapshot(txn, target_order_id)?;

        let sequence1 = self.storage.increment_sequence(txn)?;
        let sequence2 = self.storage.increment_sequence(txn)?;

        let source_table_id = source_snapshot.table_id.clone().unwrap_or_default();
        let source_table_name = source_snapshot.table_name.clone().unwrap_or_default();
        let target_table_id = target_snapshot.table_id.clone().unwrap_or_default();
        let target_table_name = target_snapshot.table_name.clone().unwrap_or_default();

        // Event for source order being merged out
        let event1 = OrderEvent::from_command(
            sequence1,
            source_order_id.to_string(),
            cmd,
            OrderEventType::OrderMergedOut,
            EventPayload::OrderMergedOut {
                target_table_id: target_table_id.clone(),
                target_table_name: target_table_name.clone(),
                reason: None,
            },
        );

        // Event for target order receiving items
        let event2 = OrderEvent::from_command(
            sequence2,
            target_order_id.to_string(),
            cmd,
            OrderEventType::OrderMerged,
            EventPayload::OrderMerged {
                source_table_id,
                source_table_name,
                items: source_snapshot.items.clone(),
            },
        );

        // Store events
        self.storage.store_event(txn, &event1)?;
        self.storage.store_event(txn, &event2)?;

        // Update snapshots
        let mut source_updated = source_snapshot.clone();
        OrderReducer::apply_event(&mut source_updated, &event1);
        self.storage.store_snapshot(txn, &source_updated)?;

        let mut target_updated = target_snapshot.clone();
        OrderReducer::apply_event(&mut target_updated, &event2);
        self.storage.store_snapshot(txn, &target_updated)?;

        // Mark source order as inactive
        self.storage.mark_order_inactive(txn, source_order_id)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event1, event2],
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_update_order_info(
        &self,
        txn: &redb::WriteTransaction,
        cmd: &OrderCommand,
        order_id: &str,
        receipt_number: Option<String>,
        guest_count: Option<i32>,
        table_name: Option<String>,
        is_pre_payment: Option<bool>,
    ) -> ManagerResult<(CommandResponse, Vec<OrderEvent>)> {
        let snapshot = self.get_active_snapshot(txn, order_id)?;
        let sequence = self.storage.increment_sequence(txn)?;

        let event = OrderEvent::from_command(
            sequence,
            order_id.to_string(),
            cmd,
            OrderEventType::OrderInfoUpdated,
            EventPayload::OrderInfoUpdated {
                receipt_number,
                guest_count,
                table_name,
                is_pre_payment,
            },
        );

        self.apply_and_store(txn, &snapshot, &event)?;

        Ok((
            CommandResponse::success(cmd.command_id.clone(), None),
            vec![event],
        ))
    }

    // ========== Helpers ==========

    fn get_active_snapshot(
        &self,
        txn: &redb::WriteTransaction,
        order_id: &str,
    ) -> ManagerResult<OrderSnapshot> {
        let snapshot = self
            .storage
            .get_snapshot_txn(txn, order_id)?
            .ok_or_else(|| ManagerError::OrderNotFound(order_id.to_string()))?;

        match snapshot.status {
            OrderStatus::Active => Ok(snapshot),
            OrderStatus::Completed => {
                Err(ManagerError::OrderAlreadyCompleted(order_id.to_string()))
            }
            OrderStatus::Void => Err(ManagerError::OrderAlreadyVoided(order_id.to_string())),
            _ => Err(ManagerError::OrderNotFound(order_id.to_string())),
        }
    }

    fn apply_and_store(
        &self,
        txn: &redb::WriteTransaction,
        snapshot: &OrderSnapshot,
        event: &OrderEvent,
    ) -> ManagerResult<()> {
        // Store event
        self.storage.store_event(txn, event)?;

        // Apply event to snapshot
        let mut updated = snapshot.clone();
        OrderReducer::apply_event(&mut updated, event);

        // Store updated snapshot
        self.storage.store_snapshot(txn, &updated)?;

        Ok(())
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
    pub fn rebuild_snapshot(&self, order_id: &str) -> ManagerResult<Option<OrderSnapshot>> {
        let events = self.storage.get_events_for_order(order_id)?;
        Ok(OrderReducer::rebuild_from_events(&events))
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
    use shared::order::PaymentInput;

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
