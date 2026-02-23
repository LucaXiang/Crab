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
use super::storage::{OrderStorage, StorageError};
use super::traits::{CommandContext, CommandHandler, CommandMetadata, EventApplier, OrderError};
use crate::order_money;
use crate::pricing::matcher::is_time_valid;
use crate::services::catalog_service::ProductMeta;
use chrono::Utc;
use chrono_tz::Tz;
use parking_lot::RwLock;
use shared::models::PriceRule;
use shared::order::types::CommandErrorCode;
use shared::order::{CommandResponse, OrderCommand, OrderEvent, OrderSnapshot, OrderStatus};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
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
    archive_service: Option<crate::archiving::OrderArchiveService>,
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
        self.archive_service = Some(crate::archiving::OrderArchiveService::new(pool, self.tz));
    }

    /// Generate next receipt number (crash-safe via redb)
    fn next_receipt_number(&self) -> String {
        let count = self.storage.next_order_count().unwrap_or(1);
        let date_str = Utc::now()
            .with_timezone(&self.tz)
            .format("%Y%m%d")
            .to_string();
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
    pub fn archive_service(&self) -> Option<&crate::archiving::OrderArchiveService> {
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
        if let shared::order::OrderCommandPayload::OpenTable {
            table_id: Some(tid),
            table_name,
            ..
        } = &cmd.payload
            && let Some(existing) = self.storage.find_active_order_for_table(*tid)?
        {
            let name = table_name.as_deref().unwrap_or("unknown");
            return Err(ManagerError::TableOccupied(format!(
                "Table {} is already occupied (order: {})",
                name, existing
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
            shared::order::OrderCommandPayload::OpenTable {
                is_retail: true, ..
            } => match self.storage.next_queue_number(self.tz) {
                Ok(qn) => {
                    tracing::debug!(queue_number = qn, "Pre-generated queue number");
                    Some(qn)
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to generate queue number");
                    None
                }
            },
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
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        "receipt_number must be pre-generated for OpenTable".to_string(),
                    )
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
                                crate::db::repository::marketing_group::find_active_rules_by_group(
                                    pool, mg_id,
                                ),
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
            shared::order::OrderCommandPayload::LinkMember {
                order_id,
                member_id,
            } => {
                // Query member info and MG rules from SQLite
                let pool = self.pool.as_ref().ok_or_else(|| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        "Database not available for member queries".to_string(),
                    )
                })?;
                let member = futures::executor::block_on(
                    crate::db::repository::member::find_member_by_id(pool, *member_id),
                )
                .map_err(|e| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        format!("Failed to query member: {e}"),
                    )
                })?
                .ok_or_else(|| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        format!("Member {} not found", member_id),
                    )
                })?;

                if !member.is_active {
                    return Err(ManagerError::from(OrderError::InvalidOperation(
                        CommandErrorCode::InvalidOperation,
                        format!("Member {} is not active", member_id),
                    )));
                }

                let mg = futures::executor::block_on(
                    crate::db::repository::marketing_group::find_by_id(
                        pool,
                        member.marketing_group_id,
                    ),
                )
                .map_err(|e| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        format!("Failed to query marketing group: {e}"),
                    )
                })?
                .ok_or_else(|| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        format!("Marketing group {} not found", member.marketing_group_id),
                    )
                })?;

                let mg_rules = futures::executor::block_on(
                    crate::db::repository::marketing_group::find_active_rules_by_group(
                        pool,
                        member.marketing_group_id,
                    ),
                )
                .map_err(|e| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        format!("Failed to query MG rules: {e}"),
                    )
                })?;

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
            shared::order::OrderCommandPayload::RedeemStamp {
                order_id,
                stamp_activity_id,
                product_id,
                comp_existing_instance_id,
            } => {
                // Query stamp activity and reward targets from SQLite
                let pool = self.pool.as_ref().ok_or_else(|| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        "Database not available for stamp queries".to_string(),
                    )
                })?;

                // Load snapshot to get member_id for stamp validation
                let snapshot = self
                    .storage
                    .get_snapshot(order_id)?
                    .ok_or_else(|| OrderError::OrderNotFound(order_id.clone()))?;
                let member_id = snapshot.member_id.ok_or_else(|| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::MemberRequired,
                        "Must have a member linked to redeem stamps".to_string(),
                    )
                })?;

                let activity = futures::executor::block_on(
                    sqlx::query_as::<_, shared::models::StampActivity>(
                        "SELECT id, marketing_group_id, name, display_name, stamps_required, reward_quantity, reward_strategy, designated_product_id, is_cyclic, is_active, created_at, updated_at FROM stamp_activity WHERE id = ? AND is_active = 1",
                    )
                    .bind(*stamp_activity_id)
                    .fetch_optional(pool),
                )
                .map_err(|e| OrderError::InvalidOperation(CommandErrorCode::InternalError, format!("Failed to query stamp activity: {e}")))?
                .ok_or_else(|| OrderError::InvalidOperation(CommandErrorCode::InternalError, format!("Stamp activity {} not found or not active", stamp_activity_id)))?;

                // Validate member has enough stamps (DB stamps + order bonus)
                let stamp_progress =
                    futures::executor::block_on(crate::db::repository::stamp::find_progress(
                        pool,
                        member_id,
                        *stamp_activity_id,
                    ))
                    .map_err(|e| {
                        OrderError::InvalidOperation(
                            CommandErrorCode::InternalError,
                            format!("Failed to query stamp progress: {e}"),
                        )
                    })?;
                let current_stamps = stamp_progress.map(|p| p.current_stamps).unwrap_or(0);

                // Count order bonus: qualifying non-comped items in the order
                let stamp_targets = futures::executor::block_on(
                    crate::db::repository::marketing_group::find_stamp_targets(
                        pool,
                        *stamp_activity_id,
                    ),
                )
                .map_err(|e| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        format!("Failed to query stamp targets: {e}"),
                    )
                })?;
                let items_with_category: Vec<_> = snapshot
                    .items
                    .iter()
                    .map(|item| crate::marketing::stamp_tracker::StampItemInfo {
                        item,
                        category_id: item.category_id,
                    })
                    .collect();
                let order_bonus = crate::marketing::stamp_tracker::count_stamps_for_order(
                    &items_with_category,
                    &stamp_targets,
                );
                let effective_stamps = current_stamps + order_bonus;

                if effective_stamps < activity.stamps_required {
                    return Err(ManagerError::InsufficientStamps {
                        current: effective_stamps,
                        required: activity.stamps_required,
                    });
                }

                // Match mode: if the comped item contributes to stamp progress, verify
                // that stamps still meet the threshold after it stops counting.
                // Only items matching stamp_targets contribute; reward-only items (e.g.
                // Designated potato when stamps come from coffee) don't reduce the count.
                if let Some(cid) = &comp_existing_instance_id
                    && let Some(comp_item) = snapshot.items.iter().find(|i| i.instance_id == *cid)
                {
                    let is_stamp_contributor = stamp_targets.iter().any(|t| match t.target_type {
                        shared::models::StampTargetType::Product => t.target_id == comp_item.id,
                        shared::models::StampTargetType::Category => {
                            comp_item.category_id == Some(t.target_id)
                        }
                    });
                    if is_stamp_contributor {
                        let post_comp_effective = effective_stamps - comp_item.quantity;
                        if post_comp_effective < activity.stamps_required {
                            return Err(ManagerError::InsufficientStamps {
                                current: post_comp_effective,
                                required: activity.stamps_required,
                            });
                        }
                    }
                }

                let mut reward_targets = futures::executor::block_on(
                    crate::db::repository::marketing_group::find_reward_targets(
                        pool,
                        *stamp_activity_id,
                    ),
                )
                .map_err(|e| {
                    OrderError::InvalidOperation(
                        CommandErrorCode::InternalError,
                        format!("Failed to query reward targets: {e}"),
                    )
                })?;

                // 留空则同计章对象: if no reward targets configured, use stamp targets
                if reward_targets.is_empty() {
                    let stamp_targets = futures::executor::block_on(
                        crate::db::repository::marketing_group::find_stamp_targets(
                            pool,
                            *stamp_activity_id,
                        ),
                    )
                    .map_err(|e| {
                        OrderError::InvalidOperation(
                            CommandErrorCode::InternalError,
                            format!("Failed to query stamp targets: {e}"),
                        )
                    })?;
                    reward_targets = stamp_targets
                        .into_iter()
                        .map(|t| shared::models::StampRewardTarget {
                            id: t.id,
                            stamp_activity_id: t.stamp_activity_id,
                            target_type: t.target_type,
                            target_id: t.target_id,
                        })
                        .collect();
                }

                // Resolve reward product info for add-new modes:
                // - Designated: use designated_product_id
                // - Selection mode (Eco/Gen + product_id): use the provided product_id
                // - Match mode (comp_existing): no product info needed (uses existing item)
                // - Auto-match mode (Eco/Gen, no product_id): resolved by action from snapshot
                let reward_product_info = if comp_existing_instance_id.is_some() {
                    None // Match mode: product info comes from existing item
                } else {
                    let pid = match activity.reward_strategy {
                        shared::models::RewardStrategy::Designated => {
                            product_id.or(activity.designated_product_id)
                        }
                        _ => *product_id, // Selection mode: explicit product_id
                    };
                    pid.and_then(|pid| {
                        let catalog = self.catalog_service.as_ref()?;
                        let product = catalog.get_product(pid)?;
                        let meta = catalog.get_product_meta(pid)?;
                        let price = product
                            .specs
                            .iter()
                            .find(|s| s.is_default)
                            .or(product.specs.first())
                            .map(|s| s.price)
                            .unwrap_or(0.0);
                        Some(super::actions::RewardProductInfo {
                            product_id: pid,
                            name: product.name,
                            price,
                            tax_rate: meta.tax_rate,
                            category_id: Some(meta.category_id),
                            category_name: Some(meta.category_name).filter(|s| !s.is_empty()),
                        })
                    })
                };

                CommandAction::RedeemStamp(super::actions::RedeemStampAction {
                    order_id: order_id.clone(),
                    stamp_activity_id: *stamp_activity_id,
                    product_id: *product_id,
                    comp_existing_instance_id: comp_existing_instance_id.clone(),
                    activity,
                    reward_targets,
                    reward_product_info,
                })
            }
            _ => (&cmd).into(),
        };
        let mut events = futures::executor::block_on(action.execute(&mut ctx, &metadata))
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

        // 6b. Auto-cancel stamp redemptions if item removal/comp reduced stamps below threshold
        let order_id_for_stamp_check: Option<&str> = match &cmd.payload {
            shared::order::OrderCommandPayload::RemoveItem { order_id, .. }
            | shared::order::OrderCommandPayload::CompItem { order_id, .. } => Some(order_id),
            _ => None,
        };
        if let Some(order_id) = order_id_for_stamp_check {
            let cancel_events =
                self.auto_cancel_invalid_stamp_redemptions(&mut ctx, &metadata, order_id)?;
            for event in &cancel_events {
                let mut snapshot = ctx
                    .load_snapshot(&event.order_id)
                    .unwrap_or_else(|_| OrderSnapshot::new(event.order_id.clone()));
                let applier: EventAction = event.into();
                applier.apply(&mut snapshot, event);
                ctx.save_snapshot(snapshot);
            }
            events.extend(cancel_events);
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
            shared::order::OrderCommandPayload::CompleteOrder { order_id, .. } => {
                self.remove_cached_rules(order_id);
                // Track stamps for completed orders with linked members
                self.track_stamps_on_completion(order_id);
            }
            shared::order::OrderCommandPayload::VoidOrder { order_id, .. } => {
                self.remove_cached_rules(order_id);
            }
            shared::order::OrderCommandPayload::MergeOrders {
                source_order_id, ..
            } => {
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

    // ========== Stamp Tracking ==========

    /// Track stamps for a completed order.
    ///
    /// Called after redb commit for CompleteOrder. If the order has a linked member,
    /// queries active stamp activities for the member's marketing group, counts matching
    /// items, and adds earned stamps to the member's progress in SQLite.
    fn track_stamps_on_completion(&self, order_id: &str) {
        let Some(pool) = &self.pool else { return };

        let snapshot = match self.storage.get_snapshot(order_id) {
            Ok(Some(s)) => s,
            _ => return,
        };

        let Some(member_id) = snapshot.member_id else {
            return;
        };
        let Some(mg_id) = snapshot.marketing_group_id else {
            return;
        };

        // Query active stamp activities for this marketing group
        let activities = match futures::executor::block_on(
            crate::db::repository::marketing_group::find_active_activities_by_group(pool, mg_id),
        ) {
            Ok(a) => a,
            Err(e) => {
                tracing::error!(order_id, error = %e, "Failed to query stamp activities for completion tracking");
                return;
            }
        };

        if activities.is_empty() {
            return;
        }

        // Build item info with category IDs from snapshot
        let items_with_category: Vec<_> = snapshot
            .items
            .iter()
            .map(|item| crate::marketing::stamp_tracker::StampItemInfo {
                item,
                category_id: item.category_id,
            })
            .collect();

        let now = shared::util::now_millis();

        for activity in &activities {
            let stamp_targets = match futures::executor::block_on(
                crate::db::repository::marketing_group::find_stamp_targets(pool, activity.id),
            ) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!(activity_id = activity.id, error = %e, "Failed to query stamp targets");
                    continue;
                }
            };

            let earned = crate::marketing::stamp_tracker::count_stamps_for_order(
                &items_with_category,
                &stamp_targets,
            );

            if earned > 0 {
                match futures::executor::block_on(crate::db::repository::stamp::add_stamps(
                    pool,
                    member_id,
                    activity.id,
                    earned,
                    now,
                )) {
                    Ok(progress) => {
                        tracing::debug!(
                            member_id,
                            activity_id = activity.id,
                            earned,
                            current = progress.current_stamps,
                            "Stamps tracked for order completion"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            member_id,
                            activity_id = activity.id,
                            error = %e,
                            "Failed to add stamps on order completion"
                        );
                    }
                }
            }
        }

        // Consume stamps for pending redemptions
        for redemption in &snapshot.stamp_redemptions {
            let Some(activity) = activities
                .iter()
                .find(|a| a.id == redemption.stamp_activity_id)
            else {
                tracing::warn!(
                    stamp_activity_id = redemption.stamp_activity_id,
                    "Stamp activity not found for redemption consumption, skipping"
                );
                continue;
            };

            match futures::executor::block_on(crate::db::repository::stamp::redeem(
                pool,
                member_id,
                activity.id,
                activity.stamps_required,
                activity.is_cyclic,
                now,
            )) {
                Ok(progress) => {
                    tracing::debug!(
                        member_id,
                        activity_id = activity.id,
                        cycles = progress.completed_cycles,
                        "Stamp redeemed on order completion"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        member_id,
                        activity_id = activity.id,
                        error = %e,
                        "Failed to redeem stamp on order completion"
                    );
                }
            }
        }
    }

    /// Auto-cancel stamp redemptions that are no longer valid after item removal/comp.
    ///
    /// When items are removed or comped, the effective stamp count may drop below the
    /// redemption threshold. This method checks each active stamp redemption and generates
    /// StampRedemptionCancelled events for those that are no longer valid.
    fn auto_cancel_invalid_stamp_redemptions(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
        order_id: &str,
    ) -> ManagerResult<Vec<OrderEvent>> {
        let pool = match &self.pool {
            Some(p) => p,
            None => return Ok(vec![]),
        };

        let snapshot = ctx.load_snapshot(order_id).map_err(ManagerError::from)?;

        if snapshot.stamp_redemptions.is_empty() {
            return Ok(vec![]);
        }

        let Some(member_id) = snapshot.member_id else {
            return Ok(vec![]);
        };

        let items_with_category: Vec<_> = snapshot
            .items
            .iter()
            .map(|item| crate::marketing::stamp_tracker::StampItemInfo {
                item,
                category_id: item.category_id,
            })
            .collect();

        let mut cancel_events = Vec::new();

        for redemption in &snapshot.stamp_redemptions {
            let activity_id = redemption.stamp_activity_id;

            let activity = futures::executor::block_on(
                sqlx::query_as::<_, shared::models::StampActivity>(
                    "SELECT id, marketing_group_id, name, display_name, stamps_required, reward_quantity, reward_strategy, designated_product_id, is_cyclic, is_active, created_at, updated_at FROM stamp_activity WHERE id = ?",
                )
                .bind(activity_id)
                .fetch_optional(pool),
            )
            .map_err(|e| ManagerError::from(OrderError::InvalidOperation(CommandErrorCode::InternalError, format!("Failed to query stamp activity: {e}"))))?;

            let Some(activity) = activity else { continue };

            let progress = futures::executor::block_on(
                crate::db::repository::stamp::find_progress(pool, member_id, activity_id),
            )
            .map_err(|e| {
                ManagerError::from(OrderError::InvalidOperation(
                    CommandErrorCode::InternalError,
                    format!("Failed to query stamp progress: {e}"),
                ))
            })?;
            let current_stamps = progress.map(|p| p.current_stamps).unwrap_or(0);

            let stamp_targets = futures::executor::block_on(
                crate::db::repository::marketing_group::find_stamp_targets(pool, activity_id),
            )
            .map_err(|e| {
                ManagerError::from(OrderError::InvalidOperation(
                    CommandErrorCode::InternalError,
                    format!("Failed to query stamp targets: {e}"),
                ))
            })?;

            let order_bonus = crate::marketing::stamp_tracker::count_stamps_for_order(
                &items_with_category,
                &stamp_targets,
            );
            let effective_stamps = current_stamps + order_bonus;

            if effective_stamps < activity.stamps_required {
                tracing::info!(
                    order_id,
                    activity_id,
                    effective_stamps,
                    required = activity.stamps_required,
                    "Auto-cancelling stamp redemption: stamps dropped below threshold"
                );

                cancel_events.push(OrderEvent::new(
                    ctx.next_sequence(),
                    order_id.to_string(),
                    metadata.operator_id,
                    metadata.operator_name.clone(),
                    metadata.command_id.clone(),
                    Some(metadata.timestamp),
                    shared::order::OrderEventType::StampRedemptionCancelled,
                    shared::order::EventPayload::StampRedemptionCancelled {
                        stamp_activity_id: activity_id,
                        stamp_activity_name: activity.display_name,
                        reward_instance_id: redemption.reward_instance_id.clone(),
                        is_comp_existing: redemption.is_comp_existing,
                        comp_source_instance_id: redemption.comp_source_instance_id.clone(),
                    },
                ));
            }
        }

        Ok(cancel_events)
    }

    // ========== Public Query Methods ==========

    /// Get a snapshot by order ID
    pub fn get_snapshot(&self, order_id: &str) -> ManagerResult<Option<OrderSnapshot>> {
        let mut snapshot = self.storage.get_snapshot(order_id)?;
        // 确保 line_total 已计算
        if let Some(ref mut order) = snapshot {
            let needs_recalc = order
                .items
                .iter()
                .any(|item| item.line_total.abs() < f64::EPSILON && !item.is_comped);
            if needs_recalc {
                order_money::recalculate_totals(order);
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
            let needs_recalc = order
                .items
                .iter()
                .any(|item| item.line_total.abs() < f64::EPSILON && !item.is_comped);
            if needs_recalc {
                order_money::recalculate_totals(order);
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

    /// Get all events for a specific order
    pub fn get_events_for_order(&self, order_id: &str) -> ManagerResult<Vec<OrderEvent>> {
        Ok(self.storage.get_events_for_order(order_id)?)
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
