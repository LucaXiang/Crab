//! redb-based storage layer for order event sourcing
//!
//! # Tables
//!
//! | Table | Key | Value | Purpose |
//! |-------|-----|-------|---------|
//! | `events` | `(order_id, sequence)` | `OrderEvent` | Event stream (append-only) |
//! | `snapshots` | `order_id` | `OrderSnapshot` | Snapshot cache |
//! | `active_orders` | `order_id` | `()` | Active order index |
//! | `processed_commands` | `command_id` | `()` | Idempotency check |
//! | `sequence_counter` | `()` | `u64` | Global sequence |
//! | `pending_archive` | `order_id` | `PendingArchive` | Archive queue |
//! | `rule_snapshots` | `order_id` | `Vec<PriceRule>` | 开台定格的价格规则快照 |
//!
//! # Durability
//!
//! Uses `WriteStrategy::TwoPhase` for maximum durability against power loss.
//! This is critical for edge devices that may experience unexpected shutdowns.
//!
//! # Snapshot Frequency
//!
//! Snapshots are persisted after every event by default. For high-throughput
//! scenarios, consider batching snapshot updates (every N events) to reduce
//! disk writes while maintaining reasonable recovery time.

use redb::{
    Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition,
    WriteTransaction,
};
use crate::db::models::PriceRule;
use shared::order::{OrderEvent, OrderSnapshot};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Table for storing events: key = (order_id, sequence), value = JSON-serialized OrderEvent
const EVENTS_TABLE: TableDefinition<(&str, u64), &[u8]> = TableDefinition::new("events");

/// Table for storing snapshots: key = order_id, value = JSON-serialized OrderSnapshot
const SNAPSHOTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("snapshots");

/// Table for tracking active orders: key = order_id, value = empty (existence check)
const ACTIVE_ORDERS_TABLE: TableDefinition<&str, ()> = TableDefinition::new("active_orders");

/// Table for tracking processed commands: key = command_id, value = empty (idempotency)
const PROCESSED_COMMANDS_TABLE: TableDefinition<&str, ()> =
    TableDefinition::new("processed_commands");

/// Table for sequence counter: key = "seq" or "order_count", value = u64
const SEQUENCE_TABLE: TableDefinition<&str, u64> = TableDefinition::new("sequence_counter");

/// Table for pending archive queue: key = order_id, value = JSON-serialized PendingArchive
const PENDING_ARCHIVE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("pending_archive");

/// Table for dead letter queue: key = order_id, value = JSON-serialized DeadLetterEntry
const DEAD_LETTER_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("dead_letter");

/// Table for price rule snapshots: key = order_id, value = JSON-serialized Vec<PriceRule>
/// 开台时定格的价格规则快照，订单生命周期内规则不变
const RULE_SNAPSHOTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("rule_snapshots");

const SEQUENCE_KEY: &str = "seq";
const ORDER_COUNT_KEY: &str = "order_count";
const QUEUE_NUMBER_KEY: &str = "queue_number";
const QUEUE_DATE_KEY: &str = "queue_date";

/// Pending archive queue entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PendingArchive {
    pub order_id: String,
    pub created_at: i64,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

/// Dead letter queue entry (permanently failed archives)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeadLetterEntry {
    pub order_id: String,
    pub created_at: i64,
    pub failed_at: i64,
    pub retry_count: u32,
    pub last_error: String,
}

/// Storage errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] redb::DatabaseError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("Table error: {0}")]
    Table(#[from] redb::TableError),

    #[error("Storage error: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("Commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Order not found: {0}")]
    OrderNotFound(String),

    #[error("Event not found: order_id={0}, sequence={1}")]
    EventNotFound(String, u64),
}

pub type StorageResult<T> = Result<T, StorageError>;

/// Order storage backed by redb
#[derive(Clone)]
pub struct OrderStorage {
    db: Arc<Database>,
}

impl OrderStorage {
    /// Open or create the database at the given path
    ///
    /// # Durability Guarantees
    ///
    /// redb uses `Durability::Immediate` by default, which ensures:
    /// - Commits are persistent as soon as `commit()` returns
    /// - Uses copy-on-write with atomic pointer swap (safe against power loss)
    /// - Database file always in consistent state
    ///
    /// This is critical for edge devices that may experience unexpected shutdowns
    /// (e.g., power outages, forced restarts).
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let db = Database::create(path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            // Create all tables if they don't exist
            let _ = write_txn.open_table(EVENTS_TABLE)?;
            let _ = write_txn.open_table(SNAPSHOTS_TABLE)?;
            let _ = write_txn.open_table(ACTIVE_ORDERS_TABLE)?;
            let _ = write_txn.open_table(PROCESSED_COMMANDS_TABLE)?;
            let _ = write_txn.open_table(PENDING_ARCHIVE_TABLE)?;
            let _ = write_txn.open_table(DEAD_LETTER_TABLE)?;
            let _ = write_txn.open_table(RULE_SNAPSHOTS_TABLE)?;

            // Initialize sequence counter if not exists
            let mut seq_table = write_txn.open_table(SEQUENCE_TABLE)?;
            if seq_table.get(SEQUENCE_KEY)?.is_none() {
                seq_table.insert(SEQUENCE_KEY, 0u64)?;
            }
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Open an in-memory database (for testing)
    #[cfg(test)]
    pub fn open_in_memory() -> StorageResult<Self> {
        let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(EVENTS_TABLE)?;
            let _ = write_txn.open_table(SNAPSHOTS_TABLE)?;
            let _ = write_txn.open_table(ACTIVE_ORDERS_TABLE)?;
            let _ = write_txn.open_table(PROCESSED_COMMANDS_TABLE)?;
            let _ = write_txn.open_table(PENDING_ARCHIVE_TABLE)?;
            let _ = write_txn.open_table(DEAD_LETTER_TABLE)?;
            let _ = write_txn.open_table(RULE_SNAPSHOTS_TABLE)?;
            let mut seq_table = write_txn.open_table(SEQUENCE_TABLE)?;
            seq_table.insert(SEQUENCE_KEY, 0u64)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Begin a write transaction
    pub fn begin_write(&self) -> StorageResult<WriteTransaction> {
        Ok(self.db.begin_write()?)
    }

    // ========== Sequence Operations ==========

    /// Get the next sequence number (does NOT increment - use within transaction)
    pub fn get_next_sequence(&self, txn: &WriteTransaction) -> StorageResult<u64> {
        let table = txn.open_table(SEQUENCE_TABLE)?;
        let current = table
            .get(SEQUENCE_KEY)?
            .map(|guard| guard.value())
            .unwrap_or(0);
        Ok(current + 1)
    }

    /// Increment and return the sequence number
    pub fn increment_sequence(&self, txn: &WriteTransaction) -> StorageResult<u64> {
        let mut table = txn.open_table(SEQUENCE_TABLE)?;
        let current = table
            .get(SEQUENCE_KEY)?
            .map(|guard| guard.value())
            .unwrap_or(0);
        let next = current + 1;
        table.insert(SEQUENCE_KEY, next)?;
        Ok(next)
    }

    /// Get current sequence (read-only)
    pub fn get_current_sequence(&self) -> StorageResult<u64> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SEQUENCE_TABLE)?;
        Ok(table
            .get(SEQUENCE_KEY)?
            .map(|guard| guard.value())
            .unwrap_or(0))
    }

    /// Set sequence number (within transaction)
    ///
    /// Used by the action-based architecture to update sequence after events are generated.
    pub fn set_sequence(&self, txn: &WriteTransaction, sequence: u64) -> StorageResult<()> {
        let mut table = txn.open_table(SEQUENCE_TABLE)?;
        table.insert(SEQUENCE_KEY, sequence)?;
        Ok(())
    }

    // ========== Order Counter (for receipt number) ==========

    /// Get and increment order count atomically
    /// Returns the NEW count after increment
    pub fn next_order_count(&self) -> StorageResult<u64> {
        let txn = self.db.begin_write()?;
        let mut table = txn.open_table(SEQUENCE_TABLE)?;
        let current = table
            .get(ORDER_COUNT_KEY)?
            .map(|g| g.value())
            .unwrap_or(0);
        let next = current + 1;
        table.insert(ORDER_COUNT_KEY, next)?;
        drop(table);
        txn.commit()?;
        Ok(next)
    }

    /// Get current order count (without incrementing)
    pub fn get_order_count(&self) -> StorageResult<u64> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SEQUENCE_TABLE)?;
        Ok(table
            .get(ORDER_COUNT_KEY)?
            .map(|g| g.value())
            .unwrap_or(0))
    }

    /// Get next queue number for retail orders (叫号)
    ///
    /// Queue number resets daily with a random start between 0-999.
    /// Wraps around after 999 back to 0.
    pub fn next_queue_number(&self, tz: chrono_tz::Tz) -> StorageResult<u32> {
        use rand::Rng;

        let today = chrono::Utc::now().with_timezone(&tz).format("%Y%m%d").to_string();
        let today_u64: u64 = today.parse().unwrap_or(0);

        let txn = self.db.begin_write()?;
        let mut table = txn.open_table(SEQUENCE_TABLE)?;

        // Check if date changed → reset with random start
        let stored_date = table.get(QUEUE_DATE_KEY)?.map(|g| g.value()).unwrap_or(0);

        let queue_num = if stored_date != today_u64 {
            // New day: random start 0-999
            let start: u64 = rand::thread_rng().gen_range(0..1000);
            table.insert(QUEUE_DATE_KEY, today_u64)?;
            table.insert(QUEUE_NUMBER_KEY, start)?;
            start
        } else {
            // Same day: increment, wrap at 1000
            let current = table
                .get(QUEUE_NUMBER_KEY)?
                .map(|g| g.value())
                .unwrap_or(0);
            let next = (current + 1) % 1000;
            table.insert(QUEUE_NUMBER_KEY, next)?;
            next
        };

        drop(table);
        txn.commit()?;
        Ok(queue_num as u32)
    }

    // ========== Command Idempotency ==========

    /// Check if a command has been processed
    pub fn is_command_processed(&self, command_id: &str) -> StorageResult<bool> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PROCESSED_COMMANDS_TABLE)?;
        Ok(table.get(command_id)?.is_some())
    }

    /// Check if a command has been processed (within transaction)
    pub fn is_command_processed_txn(
        &self,
        txn: &WriteTransaction,
        command_id: &str,
    ) -> StorageResult<bool> {
        let table = txn.open_table(PROCESSED_COMMANDS_TABLE)?;
        Ok(table.get(command_id)?.is_some())
    }

    /// Mark a command as processed
    pub fn mark_command_processed(
        &self,
        txn: &WriteTransaction,
        command_id: &str,
    ) -> StorageResult<()> {
        let mut table = txn.open_table(PROCESSED_COMMANDS_TABLE)?;
        table.insert(command_id, ())?;
        Ok(())
    }

    // ========== Event Operations ==========

    /// Store an event
    pub fn store_event(&self, txn: &WriteTransaction, event: &OrderEvent) -> StorageResult<()> {
        let mut table = txn.open_table(EVENTS_TABLE)?;
        let key = (event.order_id.as_str(), event.sequence);
        let value = serde_json::to_vec(event)?;
        table.insert(key, value.as_slice())?;
        Ok(())
    }

    /// Get all events for an order
    pub fn get_events_for_order(&self, order_id: &str) -> StorageResult<Vec<OrderEvent>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EVENTS_TABLE)?;

        let mut events = Vec::new();
        let range_start = (order_id, 0u64);
        let range_end = (order_id, u64::MAX);

        for result in table.range(range_start..=range_end)? {
            let (_key, value) = result?;
            let event: OrderEvent = serde_json::from_slice(value.value())?;
            events.push(event);
        }

        events.sort_by_key(|e| e.sequence);
        Ok(events)
    }

    /// Get events since a given sequence (across all orders)
    pub fn get_events_since(&self, since_sequence: u64) -> StorageResult<Vec<OrderEvent>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EVENTS_TABLE)?;

        let mut events = Vec::new();
        for result in table.iter()? {
            let (_key, value) = result?;
            let event: OrderEvent = serde_json::from_slice(value.value())?;
            if event.sequence > since_sequence {
                events.push(event);
            }
        }

        events.sort_by_key(|e| e.sequence);
        Ok(events)
    }

    /// Get events for active orders since a given sequence
    pub fn get_active_events_since(&self, since_sequence: u64) -> StorageResult<Vec<OrderEvent>> {
        let read_txn = self.db.begin_read()?;
        let events_table = read_txn.open_table(EVENTS_TABLE)?;
        let active_table = read_txn.open_table(ACTIVE_ORDERS_TABLE)?;

        // Get active order IDs
        let mut active_order_ids: Vec<String> = Vec::new();
        for result in active_table.iter()? {
            let (key, _value) = result?;
            active_order_ids.push(key.value().to_string());
        }

        let mut events = Vec::new();
        for order_id in &active_order_ids {
            let range_start = (order_id.as_str(), since_sequence + 1);
            let range_end = (order_id.as_str(), u64::MAX);

            for result in events_table.range(range_start..=range_end)? {
                let (_key, value) = result?;
                let event: OrderEvent = serde_json::from_slice(value.value())?;
                events.push(event);
            }
        }

        events.sort_by_key(|e| e.sequence);
        Ok(events)
    }

    // ========== Snapshot Operations ==========

    /// Store a snapshot
    pub fn store_snapshot(
        &self,
        txn: &WriteTransaction,
        snapshot: &OrderSnapshot,
    ) -> StorageResult<()> {
        let mut table = txn.open_table(SNAPSHOTS_TABLE)?;
        let value = serde_json::to_vec(snapshot)?;
        table.insert(snapshot.order_id.as_str(), value.as_slice())?;
        Ok(())
    }

    /// Get a snapshot by order ID
    pub fn get_snapshot(&self, order_id: &str) -> StorageResult<Option<OrderSnapshot>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SNAPSHOTS_TABLE)?;

        match table.get(order_id)? {
            Some(value) => {
                let snapshot: OrderSnapshot = serde_json::from_slice(value.value())?;
                Ok(Some(snapshot))
            }
            None => Ok(None),
        }
    }

    /// Get a snapshot by order ID (within transaction)
    pub fn get_snapshot_txn(
        &self,
        txn: &WriteTransaction,
        order_id: &str,
    ) -> StorageResult<Option<OrderSnapshot>> {
        let table = txn.open_table(SNAPSHOTS_TABLE)?;

        match table.get(order_id)? {
            Some(value) => {
                let snapshot: OrderSnapshot = serde_json::from_slice(value.value())?;
                Ok(Some(snapshot))
            }
            None => Ok(None),
        }
    }

    /// Get all snapshots
    pub fn get_all_snapshots(&self) -> StorageResult<Vec<OrderSnapshot>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SNAPSHOTS_TABLE)?;

        let mut snapshots = Vec::new();
        for result in table.iter()? {
            let (_key, value) = result?;
            let snapshot: OrderSnapshot = serde_json::from_slice(value.value())?;
            snapshots.push(snapshot);
        }

        Ok(snapshots)
    }

    /// Remove a snapshot
    pub fn remove_snapshot(&self, txn: &WriteTransaction, order_id: &str) -> StorageResult<()> {
        let mut table = txn.open_table(SNAPSHOTS_TABLE)?;
        table.remove(order_id)?;
        Ok(())
    }

    // ========== Active Orders ==========

    /// Mark an order as active
    pub fn mark_order_active(&self, txn: &WriteTransaction, order_id: &str) -> StorageResult<()> {
        let mut table = txn.open_table(ACTIVE_ORDERS_TABLE)?;
        table.insert(order_id, ())?;
        Ok(())
    }

    /// Mark an order as inactive
    pub fn mark_order_inactive(&self, txn: &WriteTransaction, order_id: &str) -> StorageResult<()> {
        let mut table = txn.open_table(ACTIVE_ORDERS_TABLE)?;
        table.remove(order_id)?;
        Ok(())
    }

    /// Check if an order is active
    pub fn is_order_active(&self, order_id: &str) -> StorageResult<bool> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACTIVE_ORDERS_TABLE)?;
        Ok(table.get(order_id)?.is_some())
    }

    /// Get all active order IDs
    pub fn get_active_order_ids(&self) -> StorageResult<Vec<String>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(ACTIVE_ORDERS_TABLE)?;

        let mut order_ids: Vec<String> = Vec::new();
        for result in table.iter()? {
            let (key, _value) = result?;
            order_ids.push(key.value().to_string());
        }

        Ok(order_ids)
    }

    /// Get all active order snapshots
    pub fn get_active_orders(&self) -> StorageResult<Vec<OrderSnapshot>> {
        let active_ids = self.get_active_order_ids()?;
        let mut snapshots = Vec::new();

        for order_id in active_ids {
            if let Some(snapshot) = self.get_snapshot(&order_id)? {
                snapshots.push(snapshot);
            }
        }

        Ok(snapshots)
    }

    /// Find active order for a specific table (within transaction)
    ///
    /// Returns the order_id if the table is occupied by an active order.
    pub fn find_active_order_for_table_txn(
        &self,
        txn: &WriteTransaction,
        table_id: &str,
    ) -> StorageResult<Option<String>> {
        let active_table = txn.open_table(ACTIVE_ORDERS_TABLE)?;
        let snapshots_table = txn.open_table(SNAPSHOTS_TABLE)?;

        for result in active_table.iter()? {
            let (key, _) = result?;
            let order_id = key.value();

            if let Some(value) = snapshots_table.get(order_id)? {
                let snapshot: OrderSnapshot = serde_json::from_slice(value.value())?;
                if let Some(ref tid) = snapshot.table_id
                    && tid == table_id
                {
                    return Ok(Some(order_id.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Find active order for a specific table (read-only, outside transaction)
    ///
    /// Returns the order_id if the table is occupied by an active order.
    pub fn find_active_order_for_table(&self, table_id: &str) -> StorageResult<Option<String>> {
        let read_txn = self.db.begin_read()?;
        let active_table = read_txn.open_table(ACTIVE_ORDERS_TABLE)?;
        let snapshots_table = read_txn.open_table(SNAPSHOTS_TABLE)?;

        for result in active_table.iter()? {
            let (key, _) = result?;
            let order_id = key.value();

            if let Some(value) = snapshots_table.get(order_id)? {
                let snapshot: OrderSnapshot = serde_json::from_slice(value.value())?;
                if let Some(ref tid) = snapshot.table_id
                    && tid == table_id
                {
                    return Ok(Some(order_id.to_string()));
                }
            }
        }

        Ok(None)
    }

    // ========== Rule Snapshot Operations ==========
    // 开台时定格的价格规则快照，订单生命周期内规则不变

    /// 存储订单的价格规则快照
    pub fn store_rule_snapshot(&self, order_id: &str, rules: &[PriceRule]) -> StorageResult<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(RULE_SNAPSHOTS_TABLE)?;
            let value = serde_json::to_vec(rules)?;
            table.insert(order_id, value.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }

    /// 获取订单的价格规则快照
    pub fn get_rule_snapshot(&self, order_id: &str) -> StorageResult<Option<Vec<PriceRule>>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(RULE_SNAPSHOTS_TABLE)?;

        match table.get(order_id)? {
            Some(value) => {
                let rules: Vec<PriceRule> = serde_json::from_slice(value.value())?;
                Ok(Some(rules))
            }
            None => Ok(None),
        }
    }

    /// 删除订单的价格规则快照
    pub fn remove_rule_snapshot(&self, order_id: &str) -> StorageResult<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(RULE_SNAPSHOTS_TABLE)?;
            table.remove(order_id)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// 获取所有订单的价格规则快照 (用于启动预热)
    pub fn get_all_rule_snapshots(&self) -> StorageResult<Vec<(String, Vec<PriceRule>)>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(RULE_SNAPSHOTS_TABLE)?;

        let mut snapshots = Vec::new();
        for result in table.iter()? {
            let (key, value) = result?;
            let order_id = key.value().to_string();
            let rules: Vec<PriceRule> = serde_json::from_slice(value.value())?;
            snapshots.push((order_id, rules));
        }

        Ok(snapshots)
    }

    // ========== Cleanup Operations ==========

    /// Remove events for an order (for archival)
    pub fn remove_events_for_order(
        &self,
        txn: &WriteTransaction,
        order_id: &str,
    ) -> StorageResult<Vec<OrderEvent>> {
        let mut table = txn.open_table(EVENTS_TABLE)?;

        // Collect events first
        let range_start = (order_id, 0u64);
        let range_end = (order_id, u64::MAX);

        let mut events = Vec::new();
        let mut keys_to_remove: Vec<(String, u64)> = Vec::new();

        // We need to iterate and collect separately to avoid borrow issues
        for result in table.range(range_start..=range_end)? {
            let (key, value) = result?;
            let event: OrderEvent = serde_json::from_slice(value.value())?;
            events.push(event);
            // Extract key parts - redb returns the tuple components
            let key_value = key.value();
            keys_to_remove.push((key_value.0.to_string(), key_value.1));
        }

        // Remove collected keys
        for (oid, seq) in &keys_to_remove {
            table.remove((oid.as_str(), *seq))?;
        }

        events.sort_by_key(|e| e.sequence);
        Ok(events)
    }




    /// Clean up processed command IDs for a given order
    /// (Called after archival - removes command_ids that belong to archived orders)
    pub fn cleanup_command_ids(
        &self,
        txn: &WriteTransaction,
        command_ids: &[String],
    ) -> StorageResult<()> {
        let mut table = txn.open_table(PROCESSED_COMMANDS_TABLE)?;
        for command_id in command_ids {
            table.remove(command_id.as_str())?;
        }
        Ok(())
    }

    // ========== Pending Archive Queue ==========

    /// Add order to archive queue (within transaction)
    pub fn queue_for_archive(&self, txn: &WriteTransaction, order_id: &str) -> StorageResult<()> {
        let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
        let pending = PendingArchive {
            order_id: order_id.to_string(),
            created_at: shared::util::now_millis(),
            retry_count: 0,
            last_error: None,
        };
        let value = serde_json::to_vec(&pending)?;
        table.insert(order_id, value.as_slice())?;
        Ok(())
    }

    /// Get all pending archive entries
    pub fn get_pending_archives(&self) -> StorageResult<Vec<PendingArchive>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PENDING_ARCHIVE_TABLE)?;

        let mut entries = Vec::new();
        for result in table.iter()? {
            let (_key, value) = result?;
            let pending: PendingArchive = serde_json::from_slice(value.value())?;
            entries.push(pending);
        }
        Ok(entries)
    }

    /// Complete archive: remove from pending queue and cleanup order data
    pub fn complete_archive(&self, order_id: &str) -> StorageResult<()> {
        let txn = self.begin_write()?;

        // 1. Remove from pending queue
        {
            let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
            table.remove(order_id)?;
        }

        // 2. Remove snapshot
        {
            let mut table = txn.open_table(SNAPSHOTS_TABLE)?;
            table.remove(order_id)?;
        }

        // 3. Remove events
        {
            let mut table = txn.open_table(EVENTS_TABLE)?;
            let range_start = (order_id, 0u64);
            let range_end = (order_id, u64::MAX);

            let mut keys_to_remove: Vec<(String, u64)> = Vec::new();
            for result in table.range(range_start..=range_end)? {
                let (key, _) = result?;
                let key_value = key.value();
                keys_to_remove.push((key_value.0.to_string(), key_value.1));
            }

            for (oid, seq) in &keys_to_remove {
                table.remove((oid.as_str(), *seq))?;
            }
        }

        // 4. Remove rule snapshot
        {
            let mut table = txn.open_table(RULE_SNAPSHOTS_TABLE)?;
            table.remove(order_id)?;
        }

        txn.commit()?;
        tracing::debug!(order_id = %order_id, "Archive completed, cleaned up from redb");
        Ok(())
    }

    /// Mark archive as failed, increment retry count
    pub fn mark_archive_failed(&self, order_id: &str, error: &str) -> StorageResult<()> {
        let txn = self.begin_write()?;
        {
            let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;

            // Read and clone first to avoid borrow conflict
            let pending_opt = if let Some(value) = table.get(order_id)? {
                let pending: PendingArchive = serde_json::from_slice(value.value())?;
                Some(pending)
            } else {
                None
            };

            if let Some(mut pending) = pending_opt {
                pending.retry_count += 1;
                pending.last_error = Some(error.to_string());
                let new_value = serde_json::to_vec(&pending)?;
                table.insert(order_id, new_value.as_slice())?;
            }
        }
        txn.commit()?;
        Ok(())
    }

    /// Remove from pending queue without cleanup (for dead letter)
    pub fn remove_from_pending(&self, order_id: &str) -> StorageResult<()> {
        let txn = self.begin_write()?;
        {
            let mut table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
            table.remove(order_id)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Move order from pending queue to dead letter queue
    pub fn move_to_dead_letter(&self, order_id: &str, error: &str) -> StorageResult<()> {
        let txn = self.begin_write()?;
        {
            let mut pending_table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
            let mut dead_letter_table = txn.open_table(DEAD_LETTER_TABLE)?;

            // Get pending entry
            let pending_opt = if let Some(value) = pending_table.get(order_id)? {
                let pending: PendingArchive = serde_json::from_slice(value.value())?;
                Some(pending)
            } else {
                None
            };

            if let Some(pending) = pending_opt {
                // Create dead letter entry
                let dead_letter = DeadLetterEntry {
                    order_id: order_id.to_string(),
                    created_at: pending.created_at,
                    failed_at: shared::util::now_millis(),
                    retry_count: pending.retry_count,
                    last_error: error.to_string(),
                };
                let value = serde_json::to_vec(&dead_letter)?;
                dead_letter_table.insert(order_id, value.as_slice())?;

                // Remove from pending
                pending_table.remove(order_id)?;
            }
        }
        txn.commit()?;
        Ok(())
    }

    /// Get all dead letter entries
    pub fn get_dead_letters(&self) -> StorageResult<Vec<DeadLetterEntry>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(DEAD_LETTER_TABLE)?;

        let mut entries = Vec::new();
        for result in table.iter()? {
            let (_key, value) = result?;
            let entry: DeadLetterEntry = serde_json::from_slice(value.value())?;
            entries.push(entry);
        }
        Ok(entries)
    }

    /// Remove from dead letter queue (after manual recovery)
    pub fn remove_from_dead_letter(&self, order_id: &str) -> StorageResult<()> {
        let txn = self.begin_write()?;
        {
            let mut table = txn.open_table(DEAD_LETTER_TABLE)?;
            table.remove(order_id)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Move all dead letter entries back to pending queue (reset retry count)
    ///
    /// Used at startup to retry previously failed archives after bug fixes.
    pub fn recover_dead_letters(&self) -> StorageResult<usize> {
        let txn = self.begin_write()?;
        let count = {
            let mut pending_table = txn.open_table(PENDING_ARCHIVE_TABLE)?;
            let mut dead_letter_table = txn.open_table(DEAD_LETTER_TABLE)?;

            // Collect order_ids first (can't iterate and mutate simultaneously)
            let dead_order_ids: Vec<String> = dead_letter_table.iter()?
                .filter_map(|r| r.ok())
                .map(|(k, _v)| k.value().to_string())
                .collect();

            if dead_order_ids.is_empty() {
                return Ok(0);
            }

            let now = shared::util::now_millis();
            let mut recovered = 0;
            for order_id in &dead_order_ids {
                let pending = PendingArchive {
                    order_id: order_id.clone(),
                    created_at: now,
                    retry_count: 0,
                    last_error: None,
                };
                let value = serde_json::to_vec(&pending)?;
                pending_table.insert(order_id.as_str(), value.as_slice())?;
                dead_letter_table.remove(order_id.as_str())?;
                recovered += 1;
            }
            recovered
        };
        txn.commit()?;
        Ok(count)
    }

    // ========== Statistics ==========

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageResult<StorageStats> {
        let read_txn = self.db.begin_read()?;

        let events_table = read_txn.open_table(EVENTS_TABLE)?;
        let snapshots_table = read_txn.open_table(SNAPSHOTS_TABLE)?;
        let active_table = read_txn.open_table(ACTIVE_ORDERS_TABLE)?;
        let commands_table = read_txn.open_table(PROCESSED_COMMANDS_TABLE)?;
        let seq_table = read_txn.open_table(SEQUENCE_TABLE)?;

        Ok(StorageStats {
            event_count: events_table.len()?,
            snapshot_count: snapshots_table.len()?,
            active_order_count: active_table.len()?,
            processed_command_count: commands_table.len()?,
            current_sequence: seq_table
                .get(SEQUENCE_KEY)?
                .map(|guard| guard.value())
                .unwrap_or(0),
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub event_count: u64,
    pub snapshot_count: u64,
    pub active_order_count: u64,
    pub processed_command_count: u64,
    pub current_sequence: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{EventPayload, OrderEventType, OrderStatus};

    fn create_test_event(order_id: &str, sequence: u64) -> OrderEvent {
        OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence,
            order_id: order_id.to_string(),
            timestamp: shared::util::now_millis(),
            client_timestamp: None,
            operator_id: "test_op".to_string(),
            operator_name: "Test Operator".to_string(),
            command_id: uuid::Uuid::new_v4().to_string(),
            event_type: OrderEventType::TableOpened,
            payload: EventPayload::TableOpened {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
                queue_number: None,
                receipt_number: "RCP-TEST".to_string(),
            },
        }
    }

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot {
            order_id: order_id.to_string(),
            table_id: Some("T1".to_string()),
            table_name: Some("Table 1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            service_type: None,
            queue_number: None,
            status: OrderStatus::Active,
            items: vec![],
            comps: vec![],
            payments: vec![],
            original_total: 0.0,
            subtotal: 0.0,
            total_discount: 0.0,
            total_surcharge: 0.0,
            tax: 0.0,
            discount: 0.0,
            comp_total_amount: 0.0,
            order_manual_discount_amount: 0.0,
            order_manual_surcharge_amount: 0.0,
            total: 0.0,
            paid_amount: 0.0,
            remaining_amount: 0.0,
            paid_item_quantities: std::collections::BTreeMap::new(),
            receipt_number: String::new(),
            is_pre_payment: false,
            note: None,
            order_rule_discount_amount: None,
            order_rule_surcharge_amount: None,
            order_applied_rules: None,
            order_manual_discount_percent: None,
            order_manual_discount_fixed: None,
            order_manual_surcharge_percent: None,
            order_manual_surcharge_fixed: None,
            start_time: shared::util::now_millis(),
            end_time: None,
            created_at: shared::util::now_millis(),
            updated_at: shared::util::now_millis(),
            last_sequence: 0,
            state_checksum: String::new(),
            void_type: None,
            loss_reason: None,
            loss_amount: None,
            void_note: None,
            has_amount_split: false,
            aa_total_shares: None,
            aa_paid_shares: 0,
        };
        snapshot.update_checksum();
        snapshot
    }

    #[test]
    fn test_sequence_increment() {
        let storage = OrderStorage::open_in_memory().unwrap();

        // Initial sequence should be 0
        assert_eq!(storage.get_current_sequence().unwrap(), 0);

        // Increment should return 1
        let txn = storage.begin_write().unwrap();
        let seq1 = storage.increment_sequence(&txn).unwrap();
        txn.commit().unwrap();
        assert_eq!(seq1, 1);

        // Next increment should return 2
        let txn = storage.begin_write().unwrap();
        let seq2 = storage.increment_sequence(&txn).unwrap();
        txn.commit().unwrap();
        assert_eq!(seq2, 2);

        // Current sequence should be 2
        assert_eq!(storage.get_current_sequence().unwrap(), 2);
    }

    #[test]
    fn test_command_idempotency() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let command_id = "cmd-123";

        // Command should not be processed initially
        assert!(!storage.is_command_processed(command_id).unwrap());

        // Mark as processed
        let txn = storage.begin_write().unwrap();
        storage.mark_command_processed(&txn, command_id).unwrap();
        txn.commit().unwrap();

        // Command should now be processed
        assert!(storage.is_command_processed(command_id).unwrap());
    }

    #[test]
    fn test_event_storage() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-1";

        // Store events
        let event1 = create_test_event(order_id, 1);
        let event2 = create_test_event(order_id, 2);

        let txn = storage.begin_write().unwrap();
        storage.store_event(&txn, &event1).unwrap();
        storage.store_event(&txn, &event2).unwrap();
        txn.commit().unwrap();

        // Retrieve events
        let events = storage.get_events_for_order(order_id).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
    }

    #[test]
    fn test_snapshot_storage() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-1";

        // Store snapshot
        let snapshot = create_test_snapshot(order_id);
        let txn = storage.begin_write().unwrap();
        storage.store_snapshot(&txn, &snapshot).unwrap();
        txn.commit().unwrap();

        // Retrieve snapshot
        let retrieved = storage.get_snapshot(order_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().order_id, order_id);
    }

    #[test]
    fn test_active_orders() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-1";

        // Order should not be active initially
        assert!(!storage.is_order_active(order_id).unwrap());

        // Mark as active
        let txn = storage.begin_write().unwrap();
        storage.mark_order_active(&txn, order_id).unwrap();
        txn.commit().unwrap();

        // Order should be active
        assert!(storage.is_order_active(order_id).unwrap());

        // Mark as inactive
        let txn = storage.begin_write().unwrap();
        storage.mark_order_inactive(&txn, order_id).unwrap();
        txn.commit().unwrap();

        // Order should not be active
        assert!(!storage.is_order_active(order_id).unwrap());
    }

    #[test]
    fn test_get_events_since() {
        let storage = OrderStorage::open_in_memory().unwrap();

        // Store events for multiple orders
        let event1 = create_test_event("order-1", 1);
        let event2 = create_test_event("order-2", 2);
        let event3 = create_test_event("order-1", 3);

        let txn = storage.begin_write().unwrap();
        storage.store_event(&txn, &event1).unwrap();
        storage.store_event(&txn, &event2).unwrap();
        storage.store_event(&txn, &event3).unwrap();
        txn.commit().unwrap();

        // Get events since sequence 1
        let events = storage.get_events_since(1).unwrap();
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| e.sequence > 1));
    }

    #[test]
    fn test_pending_archive_queue() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-archive-1";

        // Initially empty
        let pending = storage.get_pending_archives().unwrap();
        assert!(pending.is_empty());

        // Queue for archive
        let txn = storage.begin_write().unwrap();
        storage.queue_for_archive(&txn, order_id).unwrap();
        txn.commit().unwrap();

        // Should have one pending
        let pending = storage.get_pending_archives().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].order_id, order_id);
        assert_eq!(pending[0].retry_count, 0);
        assert!(pending[0].last_error.is_none());

        // Mark as failed
        storage.mark_archive_failed(order_id, "test error").unwrap();

        let pending = storage.get_pending_archives().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].retry_count, 1);
        assert_eq!(pending[0].last_error.as_deref(), Some("test error"));

        // Remove from pending
        storage.remove_from_pending(order_id).unwrap();

        let pending = storage.get_pending_archives().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_dead_letter_queue() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-dlq-1";

        // Initially empty
        let dead_letters = storage.get_dead_letters().unwrap();
        assert!(dead_letters.is_empty());

        // Queue for archive first
        let txn = storage.begin_write().unwrap();
        storage.queue_for_archive(&txn, order_id).unwrap();
        txn.commit().unwrap();

        // Mark as failed a few times
        storage.mark_archive_failed(order_id, "error 1").unwrap();
        storage.mark_archive_failed(order_id, "error 2").unwrap();
        storage.mark_archive_failed(order_id, "final error").unwrap();

        // Move to dead letter queue
        storage
            .move_to_dead_letter(order_id, "final error")
            .unwrap();

        // Pending should be empty
        let pending = storage.get_pending_archives().unwrap();
        assert!(pending.is_empty());

        // Dead letter should have the entry
        let dead_letters = storage.get_dead_letters().unwrap();
        assert_eq!(dead_letters.len(), 1);
        assert_eq!(dead_letters[0].order_id, order_id);
        assert_eq!(dead_letters[0].retry_count, 3);
        assert_eq!(dead_letters[0].last_error, "final error");

        // Remove from dead letter (manual recovery)
        storage.remove_from_dead_letter(order_id).unwrap();

        let dead_letters = storage.get_dead_letters().unwrap();
        assert!(dead_letters.is_empty());
    }

    #[test]
    fn test_complete_archive_cleanup() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-cleanup-1";

        // Create snapshot and events
        let snapshot = create_test_snapshot(order_id);
        let event = create_test_event(order_id, 0);

        let txn = storage.begin_write().unwrap();
        storage.store_snapshot(&txn, &snapshot).unwrap();
        storage.store_event(&txn, &event).unwrap();
        storage.queue_for_archive(&txn, order_id).unwrap();
        txn.commit().unwrap();

        // Verify data exists
        assert!(storage.get_snapshot(order_id).unwrap().is_some());
        assert!(!storage.get_events_for_order(order_id).unwrap().is_empty());
        assert!(!storage.get_pending_archives().unwrap().is_empty());

        // Complete archive (cleans up all data)
        storage.complete_archive(order_id).unwrap();

        // All data should be cleaned up
        assert!(storage.get_snapshot(order_id).unwrap().is_none());
        assert!(storage.get_events_for_order(order_id).unwrap().is_empty());
        assert!(storage.get_pending_archives().unwrap().is_empty());
    }

    // ========== Rule Snapshot Tests ==========

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
    fn test_rule_snapshot_store_and_get() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-rule-1";

        // 初始无快照
        assert!(storage.get_rule_snapshot(order_id).unwrap().is_none());

        // 存储规则快照
        let rules = vec![
            create_test_rule("Happy Hour"),
            create_test_rule("VIP Discount"),
        ];
        storage.store_rule_snapshot(order_id, &rules).unwrap();

        // 读取快照
        let retrieved = storage.get_rule_snapshot(order_id).unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].name, "Happy Hour");
        assert_eq!(retrieved[1].name, "VIP Discount");
    }

    #[test]
    fn test_rule_snapshot_overwrite() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-rule-2";

        // 第一次存储
        let rules1 = vec![create_test_rule("Rule A")];
        storage.store_rule_snapshot(order_id, &rules1).unwrap();

        // 覆盖存储
        let rules2 = vec![create_test_rule("Rule B"), create_test_rule("Rule C")];
        storage.store_rule_snapshot(order_id, &rules2).unwrap();

        let retrieved = storage.get_rule_snapshot(order_id).unwrap().unwrap();
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved[0].name, "Rule B");
    }

    #[test]
    fn test_rule_snapshot_remove() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-rule-3";

        let rules = vec![create_test_rule("Test Rule")];
        storage.store_rule_snapshot(order_id, &rules).unwrap();
        assert!(storage.get_rule_snapshot(order_id).unwrap().is_some());

        // 删除快照
        storage.remove_rule_snapshot(order_id).unwrap();
        assert!(storage.get_rule_snapshot(order_id).unwrap().is_none());

        // 删除不存在的快照不应报错
        storage.remove_rule_snapshot("nonexistent").unwrap();
    }

    #[test]
    fn test_rule_snapshot_get_all() {
        let storage = OrderStorage::open_in_memory().unwrap();

        // 初始为空
        let all = storage.get_all_rule_snapshots().unwrap();
        assert!(all.is_empty());

        // 存储多个订单的快照
        storage.store_rule_snapshot("order-a", &vec![create_test_rule("Rule A")]).unwrap();
        storage.store_rule_snapshot("order-b", &vec![create_test_rule("Rule B1"), create_test_rule("Rule B2")]).unwrap();

        let all = storage.get_all_rule_snapshots().unwrap();
        assert_eq!(all.len(), 2);

        // 验证内容（redb 按 key 排序）
        let order_ids: Vec<&str> = all.iter().map(|(id, _)| id.as_str()).collect();
        assert!(order_ids.contains(&"order-a"));
        assert!(order_ids.contains(&"order-b"));
    }

    #[test]
    fn test_rule_snapshot_empty_rules() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-empty-rules";

        // 存储空规则列表
        storage.store_rule_snapshot(order_id, &[]).unwrap();

        let retrieved = storage.get_rule_snapshot(order_id).unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_empty());
    }

    #[test]
    fn test_complete_archive_cleans_rule_snapshot() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let order_id = "order-archive-rule";

        // 创建订单数据 + 规则快照
        let snapshot = create_test_snapshot(order_id);
        let event = create_test_event(order_id, 0);
        let rules = vec![create_test_rule("Archive Rule")];

        let txn = storage.begin_write().unwrap();
        storage.store_snapshot(&txn, &snapshot).unwrap();
        storage.store_event(&txn, &event).unwrap();
        storage.queue_for_archive(&txn, order_id).unwrap();
        txn.commit().unwrap();

        storage.store_rule_snapshot(order_id, &rules).unwrap();

        // 确认规则快照存在
        assert!(storage.get_rule_snapshot(order_id).unwrap().is_some());

        // 执行归档清理
        storage.complete_archive(order_id).unwrap();

        // 规则快照也应被清理
        assert!(storage.get_rule_snapshot(order_id).unwrap().is_none());
    }
}
