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

mod error;
pub use error::*;

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
    CommandResponse, OrderCommand, OrderEvent, OrderSnapshot,
    OrderStatus,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::broadcast;

/// Event broadcast channel capacity (支持高并发: 10000订单 × 4事件)
const EVENT_CHANNEL_CAPACITY: usize = 65536;

/// Rule cache size warning threshold
const RULE_CACHE_WARN_THRESHOLD: usize = 500;

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
    /// SQLite pool for member/marketing queries (optional, only set when SQLite is available)
    pool: Option<sqlx::SqlitePool>,
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
            pool: None,
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
        self.pool = Some(pool.clone());
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
            pool: None,
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
        if cache.len() > RULE_CACHE_WARN_THRESHOLD {
            tracing::warn!(
                cache_size = cache.len(),
                "Rule cache exceeds threshold, possible order leak"
            );
        }
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
                    if self.event_tx.send(event).is_err() {
                        tracing::warn!("Event broadcast failed: no active receivers");
                        break;
                    }
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
                    if self.event_tx.send(event.clone()).is_err() {
                        tracing::warn!("Event broadcast failed: no active receivers");
                        break;
                    }
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
                    "Table {} is already occupied (order: {})", name, existing
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

                // If member is linked, get MG rules for discount calculation
                let mg_rules = if let Some(pool) = &self.pool {
                    if let Ok(Some(snapshot)) = self.storage.get_snapshot(order_id) {
                        if let Some(mg_id) = snapshot.marketing_group_id {
                            futures::executor::block_on(
                                crate::db::repository::marketing_group::find_active_rules_by_group(pool, mg_id),
                            )
                            .unwrap_or_default()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };

                CommandAction::AddItems(super::actions::AddItemsAction {
                    order_id: order_id.clone(),
                    items: items.clone(),
                    rules,
                    product_metadata,
                    mg_rules,
                })
            }
            shared::order::OrderCommandPayload::LinkMember { order_id, member_id } => {
                // Query member info and MG rules from SQLite
                let pool = self.pool.as_ref().ok_or_else(|| {
                    OrderError::InvalidOperation("Database not available for member queries".to_string())
                })?;
                let member = futures::executor::block_on(
                    crate::db::repository::member::find_member_by_id(pool, *member_id),
                )
                .map_err(|e| OrderError::InvalidOperation(format!("Failed to query member: {e}")))?
                .ok_or_else(|| OrderError::InvalidOperation(format!("Member {} not found", member_id)))?;

                if !member.is_active {
                    return Err(ManagerError::from(OrderError::InvalidOperation(
                        format!("Member {} is not active", member_id),
                    )));
                }

                let mg = futures::executor::block_on(
                    crate::db::repository::marketing_group::find_by_id(pool, member.marketing_group_id),
                )
                .map_err(|e| OrderError::InvalidOperation(format!("Failed to query marketing group: {e}")))?
                .ok_or_else(|| OrderError::InvalidOperation(format!("Marketing group {} not found", member.marketing_group_id)))?;

                let mg_rules = futures::executor::block_on(
                    crate::db::repository::marketing_group::find_active_rules_by_group(pool, member.marketing_group_id),
                )
                .map_err(|e| OrderError::InvalidOperation(format!("Failed to query MG rules: {e}")))?;

                // Get product metadata for existing items' MG scope matching
                let product_metadata = if let Some(catalog) = &self.catalog_service {
                    if let Ok(Some(snapshot)) = self.storage.get_snapshot(order_id) {
                        let product_ids: Vec<i64> = snapshot.items.iter().map(|i| i.id).collect();
                        catalog.get_product_meta_batch(&product_ids)
                    } else {
                        HashMap::new()
                    }
                } else {
                    HashMap::new()
                };

                CommandAction::LinkMember(super::actions::LinkMemberAction {
                    order_id: order_id.clone(),
                    member_id: *member_id,
                    member_name: member.name,
                    marketing_group_id: member.marketing_group_id,
                    marketing_group_name: mg.display_name,
                    mg_rules,
                    product_metadata,
                })
            }
            shared::order::OrderCommandPayload::RedeemStamp { order_id, stamp_activity_id, product_id } => {
                // Query stamp activity and reward targets from SQLite
                let pool = self.pool.as_ref().ok_or_else(|| {
                    OrderError::InvalidOperation("Database not available for stamp queries".to_string())
                })?;

                // Query the stamp activity by iterating activities of the group
                // (there is no find_activity_by_id, so we query by looking up via all activities)
                // Actually, we need a simpler approach: query activity directly
                let activity = futures::executor::block_on(
                    sqlx::query_as::<_, shared::models::StampActivity>(
                        "SELECT id, marketing_group_id, name, display_name, stamps_required, reward_quantity, reward_strategy, designated_product_id, is_cyclic, is_active, created_at, updated_at FROM stamp_activity WHERE id = ? AND is_active = 1",
                    )
                    .bind(*stamp_activity_id)
                    .fetch_optional(pool),
                )
                .map_err(|e| OrderError::InvalidOperation(format!("Failed to query stamp activity: {e}")))?
                .ok_or_else(|| OrderError::InvalidOperation(format!("Stamp activity {} not found or not active", stamp_activity_id)))?;

                let reward_targets = futures::executor::block_on(
                    crate::db::repository::marketing_group::find_reward_targets(pool, *stamp_activity_id),
                )
                .map_err(|e| OrderError::InvalidOperation(format!("Failed to query reward targets: {e}")))?;

                CommandAction::RedeemStamp(super::actions::RedeemStampAction {
                    order_id: order_id.clone(),
                    stamp_activity_id: *stamp_activity_id,
                    product_id: *product_id,
                    activity,
                    reward_targets,
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
            let needs_recalc = order.items.iter().any(|item| item.line_total.abs() < f64::EPSILON && !item.is_comped);
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
            let needs_recalc = order.items.iter().any(|item| item.line_total.abs() < f64::EPSILON && !item.is_comped);
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
            pool: self.pool.clone(),
            archive_service: self.archive_service.clone(),
            tz: self.tz,
        }
    }
}


#[cfg(test)]
mod tests;
