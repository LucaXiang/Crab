//! Order Archiving Service
//!
//! Archives completed orders from redb to SurrealDB with hash chain integrity.
//! Uses graph model with RELATE edges for items, options, payments, events.
//! All archive operations are atomic - either everything succeeds or nothing is written.

use crate::db::models::{
    Order as SurrealOrder, OrderStatus as SurrealOrderStatus, SplitItem,
};
use crate::db::repository::SystemStateRepository;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::order::{OrderEvent, OrderEventType, OrderSnapshot, OrderStatus};
use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use thiserror::Error;
use tokio::sync::{Mutex, Semaphore};

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Hash chain error: {0}")]
    HashChain(String),
    #[error("Conversion error: {0}")]
    Conversion(String),
}

pub type ArchiveResult<T> = Result<T, ArchiveError>;

// ============================================================================
// Verification Models
// ============================================================================

/// 单个事件的链路验证结果
#[derive(Debug, Serialize)]
pub struct EventVerification {
    pub event_id: String,
    pub event_type: String,
    pub expected_prev_hash: String,
    pub actual_prev_hash: String,
    pub valid: bool,
}

/// 单个订单的哈希链验证结果
#[derive(Debug, Serialize)]
pub struct OrderVerification {
    pub receipt_number: String,
    pub order_id: String,
    pub prev_hash: String,
    pub curr_hash: String,
    /// 事件链内部是否连续
    pub events_chain_valid: bool,
    pub event_count: usize,
    /// 仅包含链路断裂的事件
    pub invalid_events: Vec<EventVerification>,
}

/// 日链路验证结果
#[derive(Debug, Serialize)]
pub struct DailyChainVerification {
    pub date: String,
    pub total_orders: usize,
    pub verified_orders: usize,
    /// 整条日链是否完整（无任何断裂，含事故和损坏）
    pub chain_intact: bool,
    /// 断联事故导致的链重置（prev_hash 从 genesis 重新开始）
    /// 性质：系统事故，非数据篡改
    pub chain_resets: Vec<ChainReset>,
    /// 真正的数据损坏/篡改（prev_hash 不匹配且不是 genesis）
    pub chain_breaks: Vec<ChainBreak>,
    /// 事件链内部验证失败的订单
    pub invalid_orders: Vec<OrderVerification>,
}



/// 断联事故导致的链重置
#[derive(Debug, Serialize)]
pub struct ChainReset {
    pub receipt_number: String,
    /// 前一个订单的 curr_hash（重置前链尾的哈希）
    pub prev_chain_hash: String,
}

/// 数据损坏导致的链路断裂
#[derive(Debug, Serialize)]
pub struct ChainBreak {
    pub receipt_number: String,
    /// 上一订单的 curr_hash（期望值）
    pub expected_prev_hash: String,
    /// 该订单存储的 prev_hash（实际值）
    pub actual_prev_hash: String,
}

// ============================================================================
// DB query helper types for verification
// ============================================================================

#[derive(Debug, Deserialize)]
struct VerifyOrderRow {
    order_id: String,
    receipt_number: String,
    prev_hash: String,
    curr_hash: String,
}

#[derive(Debug, Deserialize)]
struct VerifyEventRow {
    event_id: String,
    event_type: String,
    prev_hash: String,
    curr_hash: String,
}

/// Maximum retry attempts for archiving
const MAX_RETRY_ATTEMPTS: u32 = 3;
/// Base delay between retries (exponential backoff)
const RETRY_BASE_DELAY_MS: u64 = 1000;
/// Maximum concurrent archive tasks
const MAX_CONCURRENT_ARCHIVES: usize = 5;

/// Service for archiving orders to SurrealDB
#[derive(Clone)]
pub struct OrderArchiveService {
    db: Surreal<Db>,
    system_state_repo: SystemStateRepository,
    /// Semaphore to limit concurrent archive tasks
    archive_semaphore: Arc<Semaphore>,
    /// Mutex to ensure hash chain updates are serialized (prevents race conditions)
    hash_chain_lock: Arc<Mutex<()>>,
    /// Directory for storing failed archives
    bad_archive_dir: PathBuf,
}

impl OrderArchiveService {
    pub fn new(db: Surreal<Db>) -> Self {
        let bad_archive_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("bad_archives");

        Self {
            db: db.clone(),
            system_state_repo: SystemStateRepository::new(db),
            archive_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_ARCHIVES)),
            hash_chain_lock: Arc::new(Mutex::new(())),
            bad_archive_dir,
        }
    }

    /// Generate the next receipt number atomically
    ///
    /// Format: FAC{YYYYMMDD}{sequence}
    /// Example: FAC2026012410001
    pub async fn generate_next_receipt_number(&self) -> ArchiveResult<String> {
        let next_num = self
            .system_state_repo
            .get_next_order_number()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let now = chrono::Local::now();
        let date_str = now.format("%Y%m%d").to_string();
        // Sequence starts at 10001 to match existing format
        let sequence = 10000 + next_num;
        Ok(format!("FAC{}{}", date_str, sequence))
    }

    /// Archive a completed order with its events (with retry logic and concurrency limit)
    /// Uses a single atomic transaction - either everything succeeds or nothing is written.
    /// On complete failure, saves data to bad archive file for manual recovery.
    pub async fn archive_order(
        &self,
        snapshot: &OrderSnapshot,
        events: Vec<OrderEvent>,
    ) -> ArchiveResult<()> {
        // Acquire semaphore permit to limit concurrent archives
        let _permit = self.archive_semaphore.acquire().await.map_err(|_| {
            ArchiveError::Database("Archive semaphore closed".to_string())
        })?;

        let mut last_error = None;

        for attempt in 0..MAX_RETRY_ATTEMPTS {
            match self.archive_order_internal(snapshot, &events).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::error!(
                        order_id = %snapshot.order_id,
                        error = %e,
                        attempt = attempt + 1,
                        "Archive failed"
                    );
                    last_error = Some(e);
                    if attempt + 1 < MAX_RETRY_ATTEMPTS {
                        let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                        tracing::warn!(
                            order_id = %snapshot.order_id,
                            delay_ms = delay_ms,
                            "Retrying..."
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }

        // All retries failed - save to bad archive file
        let error = last_error.unwrap_or_else(|| ArchiveError::Database("Unknown error".to_string()));
        self.save_to_bad_archive_sync(snapshot, &events, &error);
        Err(error)
    }

    /// Save failed archive data to a JSON file for manual recovery
    fn save_to_bad_archive_sync(
        &self,
        snapshot: &OrderSnapshot,
        events: &[OrderEvent],
        error: &ArchiveError,
    ) {
        #[derive(serde::Serialize)]
        struct BadArchive {
            snapshot: OrderSnapshot,
            events: Vec<OrderEvent>,
            error: String,
            timestamp: String,
        }

        let bad_archive = BadArchive {
            snapshot: snapshot.clone(),
            events: events.to_vec(),
            error: error.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Create directory if needed
        if let Err(e) = std::fs::create_dir_all(&self.bad_archive_dir) {
            tracing::error!(error = %e, "Failed to create bad archive directory");
            return;
        }

        let filename = format!(
            "{}-{}.json",
            chrono::Utc::now().format("%Y%m%d%H%M%S"),
            snapshot.order_id
        );
        let path = self.bad_archive_dir.join(&filename);

        match serde_json::to_string_pretty(&bad_archive) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, &json) {
                    tracing::error!(error = %e, path = ?path, "Failed to write bad archive file");
                } else {
                    tracing::warn!(
                        order_id = %snapshot.order_id,
                        path = ?path,
                        "Order saved to bad archive file for manual recovery"
                    );
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize bad archive");
            }
        }
    }

    /// Internal archive implementation (single attempt, atomic transaction)
    async fn archive_order_internal(
        &self,
        snapshot: &OrderSnapshot,
        events: &[OrderEvent],
    ) -> ArchiveResult<()> {
        // 0. Check idempotency - skip if already archived
        let receipt = snapshot.receipt_number.clone();
        let exists: Option<bool> = self
            .db
            .query("SELECT count() > 0 AS exists FROM order WHERE receipt_number = $receipt GROUP ALL")
            .bind(("receipt", receipt.clone()))
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .take::<Option<serde_json::Value>>(0)
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .and_then(|v| v.get("exists").and_then(|e| e.as_bool()));

        if exists == Some(true) {
            tracing::info!(order_id = %snapshot.order_id, "Order already archived, skipping");
            return Ok(());
        }

        // Acquire hash chain lock to prevent concurrent hash chain corruption
        let _hash_lock = self.hash_chain_lock.lock().await;

        // 1. Get last order hash from system_state
        let system_state = self
            .system_state_repo
            .get_or_create()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let prev_hash = system_state
            .last_order_hash
            .unwrap_or_else(|| "genesis".to_string());

        // 2. Compute order hash (includes last event hash with payload)
        let last_event_hash = events
            .last()
            .map(|e| self.compute_event_hash(e))
            .unwrap_or_else(|| "no_events".to_string());

        let order_hash = self.compute_order_hash(snapshot, &prev_hash, &last_event_hash);

        // 3. Get operator info from last event (OrderCompleted or OrderVoided)
        let (operator_id, operator_name) = events
            .iter()
            .rev()
            .find(|e| {
                matches!(
                    e.event_type,
                    OrderEventType::OrderCompleted | OrderEventType::OrderVoided
                )
            })
            .map(|e| (Some(e.operator_id.clone()), Some(e.operator_name.clone())))
            .unwrap_or((None, None));

        // 4. Convert snapshot to order struct for building query
        tracing::info!(
            order_id = %snapshot.order_id,
            status = ?snapshot.status,
            void_type = ?snapshot.void_type,
            loss_reason = ?snapshot.loss_reason,
            loss_amount = ?snapshot.loss_amount,
            "Archive: snapshot void metadata before conversion"
        );

        let surreal_order =
            self.convert_snapshot_to_order(snapshot, prev_hash, order_hash.clone(), operator_id, operator_name)?;

        tracing::info!(
            order_id = %snapshot.order_id,
            void_type = ?surreal_order.void_type,
            loss_reason = ?surreal_order.loss_reason,
            loss_amount = ?surreal_order.loss_amount,
            "Archive: converted SurrealOrder void metadata"
        );

        tracing::info!(
            order_id = %snapshot.order_id,
            items_count = snapshot.items.len(),
            payments_count = snapshot.payments.len(),
            events_count = events.len(),
            "Archiving order to SurrealDB (atomic transaction)"
        );

        // 5. Build and execute atomic transaction
        self.execute_archive_transaction(snapshot, events, &surreal_order, &order_hash).await?;

        tracing::info!(order_id = %snapshot.order_id, "Order archived to SurrealDB (graph model)");
        Ok(())
    }

    /// Execute the entire archive as a single atomic transaction
    async fn execute_archive_transaction(
        &self,
        snapshot: &OrderSnapshot,
        events: &[OrderEvent],
        order: &SurrealOrder,
        order_hash: &str,
    ) -> ArchiveResult<()> {
        // Build a single transaction query
        let mut query = String::from("BEGIN TRANSACTION;\n");

        // Create order
        query.push_str(
            r#"
            LET $order = CREATE order SET
                receipt_number = $receipt_number,
                zone_name = $zone_name,
                table_name = $table_name,
                status = $status,
                is_retail = $is_retail,
                guest_count = $guest_count,
                original_total = $original_total,
                subtotal = $subtotal,
                total_amount = $total_amount,
                paid_amount = $paid_amount,
                discount_amount = $discount_amount,
                surcharge_amount = $surcharge_amount,
                tax = $tax,
                start_time = <datetime>$start_time,
                end_time = <datetime>$end_time,
                operator_id = $operator_id,
                operator_name = $operator_name,
                void_type = $void_type,
                loss_reason = $loss_reason,
                loss_amount = $loss_amount,
                void_note = $void_note,
                prev_hash = $prev_hash,
                curr_hash = $curr_hash,
                created_at = time::now();
            "#,
        );

        // For each item, create item and its options, then RELATE
        for (i, item) in snapshot.items.iter().enumerate() {
            let var_name = format!("$item{}", i);
            query.push_str(&format!(
                r#"
                LET {} = CREATE order_item SET
                    spec = $item{}_spec,
                    instance_id = $item{}_instance_id,
                    name = $item{}_name,
                    spec_name = $item{}_spec_name,
                    price = $item{}_price,
                    quantity = $item{}_quantity,
                    unpaid_quantity = $item{}_unpaid_quantity,
                    unit_price = $item{}_unit_price,
                    line_total = $item{}_line_total,
                    discount_amount = $item{}_discount_amount,
                    surcharge_amount = $item{}_surcharge_amount,
                    tax = $item{}_tax,
                    tax_rate = $item{}_tax_rate,
                    note = $item{}_note;
                RELATE ($order[0].id)->has_item->({}[0].id);
                "#,
                var_name,
                i, i, i, i, i, i, i, i, i, i, i, i, i, i,
                var_name
            ));

            // Create options for this item
            if let Some(options) = &item.selected_options {
                for (j, _opt) in options.iter().enumerate() {
                    query.push_str(&format!(
                        r#"
                        LET $item{}_opt{} = CREATE order_item_option SET
                            attribute_name = $item{}_opt{}_attr,
                            option_name = $item{}_opt{}_name,
                            price = $item{}_opt{}_price;
                        RELATE ({}[0].id)->has_option->($item{}_opt{}[0].id);
                        "#,
                        i, j, i, j, i, j, i, j, var_name, i, j
                    ));
                }
            }
        }

        // For each payment, create and RELATE
        for i in 0..snapshot.payments.len() {
            query.push_str(&format!(
                r#"
                LET $payment{} = CREATE order_payment SET
                    method = $payment{}_method,
                    amount = $payment{}_amount,
                    time = <datetime>$payment{}_time,
                    reference = $payment{}_reference,
                    cancelled = $payment{}_cancelled,
                    cancel_reason = $payment{}_cancel_reason,
                    split_items = $payment{}_split_items;
                RELATE ($order[0].id)->has_payment->($payment{}[0].id);
                "#,
                i, i, i, i, i, i, i, i, i
            ));
        }

        // For each event, create and RELATE
        for i in 0..events.len() {
            query.push_str(&format!(
                r#"
                LET $event{} = CREATE order_event SET
                    event_type = $event{}_type,
                    timestamp = <datetime>$event{}_timestamp,
                    data = $event{}_data,
                    prev_hash = $event{}_prev_hash,
                    curr_hash = $event{}_curr_hash;
                RELATE ($order[0].id)->has_event->($event{}[0].id);
                "#,
                i, i, i, i, i, i, i
            ));
        }

        // Update system_state (use UPSERT to ensure record exists)
        query.push_str(
            r#"
            UPSERT system_state:main SET
                last_order = $order[0].id,
                last_order_hash = $order_hash,
                updated_at = time::now();
            COMMIT TRANSACTION;
            RETURN { success: true, order_id: <string>$order[0].id };
            "#,
        );

        // Build the query with all bindings
        let mut db_query = self.db.query(&query);

        // Bind order fields
        let status_str = match order.status {
            SurrealOrderStatus::Completed => "COMPLETED",
            SurrealOrderStatus::Void => "VOID",
            SurrealOrderStatus::Moved => "MOVED",
            SurrealOrderStatus::Merged => "MERGED",
        };

        db_query = db_query
            .bind(("receipt_number", order.receipt_number.clone()))
            .bind(("zone_name", order.zone_name.clone()))
            .bind(("table_name", order.table_name.clone()))
            .bind(("status", status_str))
            .bind(("is_retail", order.is_retail))
            .bind(("guest_count", order.guest_count))
            .bind(("original_total", order.original_total))
            .bind(("subtotal", order.subtotal))
            .bind(("total_amount", order.total_amount))
            .bind(("paid_amount", order.paid_amount))
            .bind(("discount_amount", order.discount_amount))
            .bind(("surcharge_amount", order.surcharge_amount))
            .bind(("tax", order.tax))
            .bind(("start_time", order.start_time.clone()))
            .bind(("end_time", order.end_time.clone()))
            .bind(("operator_id", order.operator_id.clone()))
            .bind(("operator_name", order.operator_name.clone()))
            .bind(("void_type", order.void_type.clone()))
            .bind(("loss_reason", order.loss_reason.clone()))
            .bind(("loss_amount", order.loss_amount))
            .bind(("void_note", order.void_note.clone()))
            .bind(("prev_hash", order.prev_hash.clone()))
            .bind(("curr_hash", order.curr_hash.clone()))
            .bind(("order_hash", order_hash.to_string()));

        // Bind item fields
        for (i, item) in snapshot.items.iter().enumerate() {
            let base_price = item.original_price.unwrap_or(item.price);
            let manual_discount_per_unit = item
                .manual_discount_percent
                .map(|p| base_price * p / 100.0)
                .unwrap_or(0.0);
            let rule_discount_per_unit = item.rule_discount_amount.unwrap_or(0.0);
            let total_discount = (manual_discount_per_unit + rule_discount_per_unit) * item.quantity as f64;
            let surcharge_per_unit = item.surcharge.unwrap_or(0.0) + item.rule_surcharge_amount.unwrap_or(0.0);
            let total_surcharge = surcharge_per_unit * item.quantity as f64;
            let unit_price = item.unit_price.unwrap_or(base_price - manual_discount_per_unit - rule_discount_per_unit + surcharge_per_unit);
            let line_total = item.line_total.unwrap_or(unit_price * item.quantity as f64);
            let spec_name = item.selected_specification.as_ref().map(|s| s.name.clone());
            let instance_id = item.instance_id.clone();
            let paid_qty = snapshot.paid_item_quantities.get(&instance_id).copied().unwrap_or(0);
            let unpaid_quantity = (item.quantity - paid_qty).max(0);

            db_query = db_query
                .bind((format!("item{}_spec", i), item.id.clone()))
                .bind((format!("item{}_instance_id", i), instance_id))
                .bind((format!("item{}_name", i), item.name.clone()))
                .bind((format!("item{}_spec_name", i), spec_name))
                .bind((format!("item{}_price", i), base_price))
                .bind((format!("item{}_quantity", i), item.quantity))
                .bind((format!("item{}_unpaid_quantity", i), unpaid_quantity))
                .bind((format!("item{}_unit_price", i), unit_price))
                .bind((format!("item{}_line_total", i), line_total))
                .bind((format!("item{}_discount_amount", i), total_discount))
                .bind((format!("item{}_surcharge_amount", i), total_surcharge))
                .bind((format!("item{}_tax", i), item.tax.unwrap_or(0.0)))
                .bind((format!("item{}_tax_rate", i), item.tax_rate.unwrap_or(0)))
                .bind((format!("item{}_note", i), item.note.clone()));

            // Bind option fields
            if let Some(options) = &item.selected_options {
                for (j, opt) in options.iter().enumerate() {
                    db_query = db_query
                        .bind((format!("item{}_opt{}_attr", i, j), opt.attribute_name.clone()))
                        .bind((format!("item{}_opt{}_name", i, j), opt.option_name.clone()))
                        .bind((format!("item{}_opt{}_price", i, j), opt.price_modifier.unwrap_or(0.0)));
                }
            }
        }

        // Bind payment fields
        for (i, payment) in snapshot.payments.iter().enumerate() {
            let time = chrono::DateTime::from_timestamp_millis(payment.timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            // Serialize split_items to JSON string (SurrealDB SDK has issues with Vec binding)
            let split_items_str: Option<String> = payment
                .split_items
                .as_ref()
                .map(|items| {
                    let split_items: Vec<SplitItem> = items.iter().map(|si| SplitItem {
                        instance_id: si.instance_id.clone(),
                        name: si.name.clone(),
                        quantity: si.quantity,
                        unit_price: si.unit_price.unwrap_or(si.price),
                    }).collect();
                    serde_json::to_string(&split_items).unwrap_or_else(|_| "[]".to_string())
                });

            db_query = db_query
                .bind((format!("payment{}_method", i), payment.method.clone()))
                .bind((format!("payment{}_amount", i), payment.amount))
                .bind((format!("payment{}_time", i), time))
                .bind((format!("payment{}_reference", i), payment.note.clone()))
                .bind((format!("payment{}_cancelled", i), payment.cancelled))
                .bind((format!("payment{}_cancel_reason", i), payment.cancel_reason.clone()))
                .bind((format!("payment{}_split_items", i), split_items_str));
        }

        // Bind event fields
        for (i, event) in events.iter().enumerate() {
            let prev_event_hash = if i == 0 {
                "order_start".to_string()
            } else {
                self.compute_event_hash(&events[i - 1])
            };
            let curr_event_hash = self.compute_event_hash(event);

            // Use serde to get correct SCREAMING_SNAKE_CASE format (e.g., "TABLE_OPENED")
            let event_type_str = serde_json::to_value(&event.event_type)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("{:?}", event.event_type).to_uppercase());

            // Convert timestamp (millis) to RFC3339 for SurrealDB datetime
            let timestamp_str = chrono::DateTime::from_timestamp_millis(event.timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

            // Serialize payload to JSON string
            let payload_str = serde_json::to_string(&event.payload)
                .unwrap_or_else(|_| "{}".to_string());

            db_query = db_query
                .bind((format!("event{}_type", i), event_type_str))
                .bind((format!("event{}_timestamp", i), timestamp_str))
                .bind((format!("event{}_data", i), payload_str))
                .bind((format!("event{}_prev_hash", i), prev_event_hash))
                .bind((format!("event{}_curr_hash", i), curr_event_hash));
        }

        // Execute the transaction
        let result = db_query.await.map_err(|e| ArchiveError::Database(e.to_string()))?;

        // Check for errors in response
        let errors = result.check();
        if let Err(e) = errors {
            return Err(ArchiveError::Database(e.to_string()));
        }

        Ok(())
    }

    fn compute_order_hash(
        &self,
        snapshot: &OrderSnapshot,
        prev_hash: &str,
        last_event_hash: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(snapshot.order_id.as_bytes());
        hasher.update(snapshot.receipt_number.as_bytes());
        hasher.update(format!("{:?}", snapshot.status).as_bytes());
        hasher.update(last_event_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute event hash including payload for tamper-proofing
    fn compute_event_hash(&self, event: &OrderEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event.event_id.as_bytes());
        hasher.update(event.order_id.as_bytes());
        hasher.update(format!("{}", event.sequence).as_bytes());
        hasher.update(format!("{:?}", event.event_type).as_bytes());
        // Include payload in hash for tamper-proofing
        let payload_json = serde_json::to_string(&event.payload).unwrap_or_default();
        hasher.update(payload_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // ========================================================================
    // Verification Methods
    // ========================================================================

    /// 验证单个订单的事件哈希链完整性
    pub async fn verify_order(&self, receipt_number: &str) -> ArchiveResult<OrderVerification> {
        // 1. 查询订单
        let mut result = self
            .db
            .query(
                r#"
                SELECT
                    <string>id AS order_id,
                    receipt_number,
                    prev_hash,
                    curr_hash
                FROM order
                WHERE receipt_number = $receipt
                "#,
            )
            .bind(("receipt", receipt_number.to_string()))
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let order: VerifyOrderRow = result
            .take::<Vec<VerifyOrderRow>>(0)
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| {
                ArchiveError::Database(format!("Order not found: {}", receipt_number))
            })?;

        // 2. 查询该订单的所有事件（按 timestamp 排序）
        //    使用 record ID 变量绑定，通过 graph traversal 查询
        let order_record_id: surrealdb::RecordId = order
            .order_id
            .parse()
            .map_err(|e: surrealdb::Error| ArchiveError::Database(e.to_string()))?;

        let mut event_result = self
            .db
            .query(
                r#"
                SELECT
                    <string>id AS event_id,
                    event_type,
                    prev_hash,
                    curr_hash
                FROM $order_id->has_event->order_event
                ORDER BY timestamp
                "#,
            )
            .bind(("order_id", order_record_id))
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let events: Vec<VerifyEventRow> = event_result
            .take(0)
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 3. 验证事件链连续性
        let mut invalid_events = Vec::new();
        let mut events_chain_valid = true;

        for (i, event) in events.iter().enumerate() {
            let expected_prev = if i == 0 {
                "order_start".to_string()
            } else {
                events[i - 1].curr_hash.clone()
            };

            if event.prev_hash != expected_prev {
                events_chain_valid = false;
                invalid_events.push(EventVerification {
                    event_id: event.event_id.clone(),
                    event_type: event.event_type.clone(),
                    expected_prev_hash: expected_prev,
                    actual_prev_hash: event.prev_hash.clone(),
                    valid: false,
                });
            }
        }

        Ok(OrderVerification {
            receipt_number: order.receipt_number,
            order_id: order.order_id,
            prev_hash: order.prev_hash,
            curr_hash: order.curr_hash,
            events_chain_valid,
            event_count: events.len(),
            invalid_events,
        })
    }

    /// 验证指定时间范围内所有订单的哈希链连续性
    ///
    /// - `date`: 标签（用于返回值，如 "2026-01-29"）
    /// - `start`/`end`: ISO 8601 格式的时间范围（由 handler 层根据 business_day_cutoff 计算）
    pub async fn verify_daily_chain(
        &self,
        date: &str,
        start: &str,
        end: &str,
    ) -> ArchiveResult<DailyChainVerification> {

        // 1. 查询当天所有订单，按 created_at 排序
        let mut result = self
            .db
            .query(
                r#"
                SELECT
                    <string>id AS order_id,
                    receipt_number,
                    prev_hash,
                    curr_hash
                FROM order
                WHERE created_at >= <datetime>$start AND created_at < <datetime>$end
                ORDER BY created_at
                "#,
            )
            .bind(("start", start.to_string()))
            .bind(("end", end.to_string()))
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let orders: Vec<VerifyOrderRow> = result
            .take(0)
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let total_orders = orders.len();

        // 2. 查询范围之前最后一个订单的 curr_hash
        let mut prev_result = self
            .db
            .query(
                r#"
                SELECT curr_hash
                FROM order
                WHERE created_at < <datetime>$start
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(("start", start.to_string()))
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        #[derive(Debug, Deserialize)]
        struct HashRow {
            curr_hash: String,
        }

        let prev_day_hash = prev_result
            .take::<Vec<HashRow>>(0)
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .into_iter()
            .next()
            .map(|r| r.curr_hash);

        // 确定当天第一个订单的 expected prev_hash
        // 有前一天订单 → 用其 curr_hash；没有 → 用第一个订单自身的 prev_hash（不猜）
        let expected_first_prev = prev_day_hash.unwrap_or_else(|| {
            orders
                .first()
                .map(|o| o.prev_hash.clone())
                .unwrap_or_else(|| "genesis".to_string())
        });

        // 3. 遍历验证
        let mut chain_intact = true;
        let mut chain_resets: Vec<ChainReset> = Vec::new();
        let mut chain_breaks: Vec<ChainBreak> = Vec::new();
        let mut invalid_orders = Vec::new();
        let mut verified_orders = 0usize;
        let mut expected_prev = expected_first_prev;

        for order in &orders {
            verified_orders += 1;

            if order.prev_hash != expected_prev {
                chain_intact = false;
                if order.prev_hash == "genesis" {
                    // 断联事故：system_state 丢失后链从 genesis 重新开始
                    chain_resets.push(ChainReset {
                        receipt_number: order.receipt_number.clone(),
                        prev_chain_hash: expected_prev.clone(),
                    });
                } else {
                    // 数据损坏：prev_hash 既不匹配也不是 genesis
                    chain_breaks.push(ChainBreak {
                        receipt_number: order.receipt_number.clone(),
                        expected_prev_hash: expected_prev.clone(),
                        actual_prev_hash: order.prev_hash.clone(),
                    });
                }
            }

            // 验证订单内部事件链
            let order_verification = self.verify_order(&order.receipt_number).await?;
            if !order_verification.events_chain_valid {
                chain_intact = false;
                invalid_orders.push(order_verification);
            }

            expected_prev = order.curr_hash.clone();
        }

        Ok(DailyChainVerification {
            date: date.to_string(),
            total_orders,
            verified_orders,
            chain_intact,
            chain_resets,
            chain_breaks,
            invalid_orders,
        })
    }



    fn convert_snapshot_to_order(
        &self,
        snapshot: &OrderSnapshot,
        prev_hash: String,
        curr_hash: String,
        operator_id: Option<String>,
        operator_name: Option<String>,
    ) -> ArchiveResult<SurrealOrder> {
        let status = match snapshot.status {
            OrderStatus::Completed => SurrealOrderStatus::Completed,
            OrderStatus::Void => SurrealOrderStatus::Void,
            OrderStatus::Moved => SurrealOrderStatus::Moved,
            OrderStatus::Merged => SurrealOrderStatus::Merged,
            _ => {
                return Err(ArchiveError::Conversion(format!(
                    "Cannot archive order with status {:?}",
                    snapshot.status
                )))
            }
        };

        Ok(SurrealOrder {
            id: None,
            receipt_number: snapshot.receipt_number.clone(),
            zone_name: snapshot.zone_name.clone(),
            table_name: snapshot.table_name.clone(),
            status,
            is_retail: snapshot.is_retail,
            guest_count: Some(snapshot.guest_count),
            original_total: snapshot.original_total,
            subtotal: snapshot.subtotal,
            total_amount: snapshot.total,
            paid_amount: snapshot.paid_amount,
            discount_amount: snapshot.total_discount,
            surcharge_amount: snapshot.total_surcharge,
            tax: snapshot.tax,
            start_time: chrono::DateTime::from_timestamp_millis(snapshot.start_time)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
            end_time: snapshot.end_time.map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }),
            void_type: snapshot.void_type.as_ref().map(|v| {
                serde_json::to_value(v).ok()
                    .and_then(|val| val.as_str().map(String::from))
                    .unwrap_or_default()
            }),
            loss_reason: snapshot.loss_reason.as_ref().map(|r| {
                serde_json::to_value(r).ok()
                    .and_then(|val| val.as_str().map(String::from))
                    .unwrap_or_default()
            }),
            loss_amount: snapshot.loss_amount,
            void_note: snapshot.void_note.clone(),
            operator_id,
            operator_name,
            prev_hash,
            curr_hash,
            related_order_id: None,
            created_at: None,
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{OrderEventType, OrderSnapshot, OrderStatus};

    fn create_test_snapshot() -> OrderSnapshot {
        OrderSnapshot {
            order_id: "test-order-1".to_string(),
            table_id: Some("T1".to_string()),
            table_name: Some("Table 1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            status: OrderStatus::Completed,
            void_type: None,
            loss_reason: None,
            loss_amount: None,
            void_note: None,
            items: vec![],
            payments: vec![],
            original_total: 100.0,
            subtotal: 100.0,
            total_discount: 0.0,
            total_surcharge: 0.0,
            tax: 0.0,
            discount: 0.0,
            total: 100.0,
            paid_amount: 100.0,
            remaining_amount: 0.0,
            paid_item_quantities: std::collections::HashMap::new(),
            receipt_number: "R001".to_string(),
            is_pre_payment: false,
            order_rule_discount_amount: None,
            order_rule_surcharge_amount: None,
            order_applied_rules: None,
            order_manual_discount_percent: None,
            order_manual_discount_fixed: None,
            start_time: 1704067200000,
            end_time: Some(1704070800000),
            created_at: 1704067200000,
            updated_at: 1704070800000,
            last_sequence: 5,
            state_checksum: String::new(),
            has_amount_split: false,
            aa_total_shares: None,
            aa_paid_shares: 0,
        }
    }

    fn create_test_event(order_id: &str, sequence: u64) -> shared::order::OrderEvent {
        shared::order::OrderEvent {
            event_id: format!("event-{}", sequence),
            sequence,
            order_id: order_id.to_string(),
            timestamp: 1704067200000,
            client_timestamp: None,
            operator_id: "op-1".to_string(),
            operator_name: "Test Operator".to_string(),
            command_id: format!("cmd-{}", sequence),
            event_type: OrderEventType::TableOpened,
            payload: shared::order::EventPayload::TableOpened {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
                receipt_number: "RCP-TEST".to_string(),
            },
        }
    }

    #[test]
    fn test_compute_order_hash_deterministic() {
        let snapshot = create_test_snapshot();

        let hash1 = compute_order_hash_standalone(&snapshot, "prev_hash", "event_hash");
        let hash2 = compute_order_hash_standalone(&snapshot, "prev_hash", "event_hash");

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_compute_order_hash_different_inputs() {
        let snapshot = create_test_snapshot();

        let hash1 = compute_order_hash_standalone(&snapshot, "prev_hash_a", "event_hash");
        let hash2 = compute_order_hash_standalone(&snapshot, "prev_hash_b", "event_hash");

        assert_ne!(hash1, hash2); // Different prev_hash should produce different hash
    }

    #[test]
    fn test_compute_event_hash_deterministic() {
        let event = create_test_event("order-1", 1);

        let hash1 = compute_event_hash_standalone(&event);
        let hash2 = compute_event_hash_standalone(&event);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_compute_event_hash_includes_payload() {
        let event1 = create_test_event("order-1", 1);
        let mut event2 = create_test_event("order-1", 1);

        // Modify payload
        event2.payload = shared::order::EventPayload::TableOpened {
            table_id: Some("T2".to_string()), // Different table
            table_name: Some("Table 2".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            receipt_number: "RCP-TEST".to_string(),
        };

        let hash1 = compute_event_hash_standalone(&event1);
        let hash2 = compute_event_hash_standalone(&event2);

        assert_ne!(hash1, hash2); // Different payload should produce different hash
    }

    // Standalone functions for testing without OrderArchiveService
    fn compute_order_hash_standalone(
        snapshot: &OrderSnapshot,
        prev_hash: &str,
        last_event_hash: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(snapshot.order_id.as_bytes());
        hasher.update(snapshot.receipt_number.as_bytes());
        hasher.update(format!("{:?}", snapshot.status).as_bytes());
        hasher.update(last_event_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn compute_event_hash_standalone(event: &shared::order::OrderEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event.event_id.as_bytes());
        hasher.update(event.order_id.as_bytes());
        hasher.update(format!("{}", event.sequence).as_bytes());
        hasher.update(format!("{:?}", event.event_type).as_bytes());
        // Include payload in hash for tamper-proofing
        let payload_json = serde_json::to_string(&event.payload).unwrap_or_default();
        hasher.update(payload_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // ========================================================================
    // Chain Verification Logic Tests
    // ========================================================================

    /// Helper: build a chain of VerifyEventRow with valid prev/curr linkage
    fn build_valid_event_chain(count: usize) -> Vec<VerifyEventRow> {
        let mut events = Vec::with_capacity(count);
        let mut prev = "order_start".to_string();
        for i in 0..count {
            let curr = format!("event_hash_{}", i);
            events.push(VerifyEventRow {
                event_id: format!("order_event:{}", i),
                event_type: "TABLE_OPENED".to_string(),
                prev_hash: prev,
                curr_hash: curr.clone(),
            });
            prev = curr;
        }
        events
    }

    /// Helper: validate event chain (mirrors the logic in verify_order)
    fn validate_event_chain(events: &[VerifyEventRow]) -> (bool, Vec<EventVerification>) {
        let mut invalid = Vec::new();
        let mut valid = true;
        for (i, event) in events.iter().enumerate() {
            let expected_prev = if i == 0 {
                "order_start".to_string()
            } else {
                events[i - 1].curr_hash.clone()
            };
            if event.prev_hash != expected_prev {
                valid = false;
                invalid.push(EventVerification {
                    event_id: event.event_id.clone(),
                    event_type: event.event_type.clone(),
                    expected_prev_hash: expected_prev,
                    actual_prev_hash: event.prev_hash.clone(),
                    valid: false,
                });
            }
        }
        (valid, invalid)
    }

    #[test]
    fn test_event_chain_valid() {
        let events = build_valid_event_chain(5);
        let (valid, invalid) = validate_event_chain(&events);
        assert!(valid);
        assert!(invalid.is_empty());
    }

    #[test]
    fn test_event_chain_broken_first_event() {
        let mut events = build_valid_event_chain(3);
        // Break the first event's prev_hash (should be "order_start")
        events[0].prev_hash = "wrong_start".to_string();

        let (valid, invalid) = validate_event_chain(&events);
        assert!(!valid);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0].event_id, "order_event:0");
        assert_eq!(invalid[0].expected_prev_hash, "order_start");
        assert_eq!(invalid[0].actual_prev_hash, "wrong_start");
    }

    #[test]
    fn test_event_chain_broken_middle() {
        let mut events = build_valid_event_chain(5);
        // Break event[2]'s prev_hash
        events[2].prev_hash = "tampered".to_string();

        let (valid, invalid) = validate_event_chain(&events);
        assert!(!valid);
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0].event_id, "order_event:2");
        assert_eq!(invalid[0].expected_prev_hash, "event_hash_1");
        assert_eq!(invalid[0].actual_prev_hash, "tampered");
    }

    #[test]
    fn test_event_chain_multiple_breaks() {
        let mut events = build_valid_event_chain(5);
        events[1].prev_hash = "tampered_1".to_string();
        events[3].prev_hash = "tampered_3".to_string();

        let (valid, invalid) = validate_event_chain(&events);
        assert!(!valid);
        assert_eq!(invalid.len(), 2);
        assert_eq!(invalid[0].event_id, "order_event:1");
        assert_eq!(invalid[1].event_id, "order_event:3");
    }

    #[test]
    fn test_event_chain_empty() {
        let events: Vec<VerifyEventRow> = vec![];
        let (valid, invalid) = validate_event_chain(&events);
        assert!(valid);
        assert!(invalid.is_empty());
    }

    /// Helper: build a chain of VerifyOrderRow with valid prev/curr linkage
    fn build_valid_order_chain(count: usize, genesis: &str) -> Vec<VerifyOrderRow> {
        let mut orders = Vec::with_capacity(count);
        let mut prev = genesis.to_string();
        for i in 0..count {
            let curr = format!("order_hash_{}", i);
            orders.push(VerifyOrderRow {
                order_id: format!("order:{}", i),
                receipt_number: format!("FAC202601290{:04}", i + 1),
                prev_hash: prev,
                curr_hash: curr.clone(),
            });
            prev = curr;
        }
        orders
    }

    /// Helper: validate order chain (mirrors the logic in verify_daily_chain)
    fn validate_order_chain(
        orders: &[VerifyOrderRow],
        expected_first_prev: &str,
    ) -> (bool, Option<ChainBreak>) {
        let mut intact = true;
        let mut first_break = None;
        let mut expected_prev = expected_first_prev.to_string();

        for order in orders {
            if order.prev_hash != expected_prev {
                intact = false;
                if first_break.is_none() {
                    first_break = Some(ChainBreak {
                        receipt_number: order.receipt_number.clone(),
                        expected_prev_hash: expected_prev.clone(),
                        actual_prev_hash: order.prev_hash.clone(),
                    });
                }
            }
            expected_prev = order.curr_hash.clone();
        }

        (intact, first_break)
    }

    #[test]
    fn test_order_chain_valid_from_genesis() {
        let orders = build_valid_order_chain(3, "genesis");
        let (intact, first_break) = validate_order_chain(&orders, "genesis");
        assert!(intact);
        assert!(first_break.is_none());
    }

    #[test]
    fn test_order_chain_valid_from_previous_day() {
        let prev_day_hash = "prev_day_last_order_hash";
        let orders = build_valid_order_chain(3, prev_day_hash);
        let (intact, first_break) = validate_order_chain(&orders, prev_day_hash);
        assert!(intact);
        assert!(first_break.is_none());
    }

    #[test]
    fn test_order_chain_broken_first_order() {
        let orders = build_valid_order_chain(3, "genesis");
        // Validate with wrong expected prev — simulates missing previous day data
        let (intact, first_break) = validate_order_chain(&orders, "wrong_genesis");
        assert!(!intact);
        let brk = first_break.unwrap();
        assert_eq!(brk.receipt_number, "FAC2026012900001");
        assert_eq!(brk.expected_prev_hash, "wrong_genesis");
        assert_eq!(brk.actual_prev_hash, "genesis");
    }

    #[test]
    fn test_order_chain_broken_middle() {
        let mut orders = build_valid_order_chain(5, "genesis");
        // Tamper with order[2]'s prev_hash
        orders[2].prev_hash = "tampered".to_string();

        let (intact, first_break) = validate_order_chain(&orders, "genesis");
        assert!(!intact);
        let brk = first_break.unwrap();
        assert_eq!(brk.receipt_number, "FAC2026012900003");
        assert_eq!(brk.expected_prev_hash, "order_hash_1");
        assert_eq!(brk.actual_prev_hash, "tampered");
    }

    #[test]
    fn test_order_chain_empty() {
        let orders: Vec<VerifyOrderRow> = vec![];
        let (intact, first_break) = validate_order_chain(&orders, "genesis");
        assert!(intact);
        assert!(first_break.is_none());
    }

    #[test]
    fn test_daily_chain_verification_intact() {
        let verification = DailyChainVerification {
            date: "2026-01-29".to_string(),
            total_orders: 10,
            verified_orders: 10,
            chain_intact: true,
            chain_resets: vec![],
            chain_breaks: vec![],
            invalid_orders: vec![],
        };
        assert!(verification.chain_intact);
        assert!(verification.chain_resets.is_empty());
        assert!(verification.chain_breaks.is_empty());

        let json = serde_json::to_string(&verification).unwrap();
        assert!(json.contains("\"chain_intact\":true"));
    }

    #[test]
    fn test_daily_chain_verification_with_reset() {
        // 断联事故：O4 从 genesis 重新开始
        let verification = DailyChainVerification {
            date: "2026-01-29".to_string(),
            total_orders: 6,
            verified_orders: 6,
            chain_intact: false,
            chain_resets: vec![ChainReset {
                receipt_number: "FAC2026012910004".to_string(),
                prev_chain_hash: "ccc".to_string(),
            }],
            chain_breaks: vec![],
            invalid_orders: vec![],
        };
        assert!(!verification.chain_intact);
        assert_eq!(verification.chain_resets.len(), 1);
        assert!(verification.chain_breaks.is_empty()); // 不是数据损坏

        let json = serde_json::to_string(&verification).unwrap();
        assert!(json.contains("\"chain_resets\""));
        assert!(json.contains("\"prev_chain_hash\":\"ccc\""));
    }

    #[test]
    fn test_daily_chain_verification_with_corruption() {
        // 数据损坏：O3.prev_hash 被篡改
        let verification = DailyChainVerification {
            date: "2026-01-29".to_string(),
            total_orders: 5,
            verified_orders: 5,
            chain_intact: false,
            chain_resets: vec![],
            chain_breaks: vec![ChainBreak {
                receipt_number: "FAC2026012910003".to_string(),
                expected_prev_hash: "bbb".to_string(),
                actual_prev_hash: "tampered".to_string(),
            }],
            invalid_orders: vec![],
        };
        assert!(!verification.chain_intact);
        assert!(verification.chain_resets.is_empty());
        assert_eq!(verification.chain_breaks.len(), 1);
    }

    #[test]
    fn test_daily_chain_verification_both_issues() {
        // 同时有事故和损坏
        let verification = DailyChainVerification {
            date: "2026-01-29".to_string(),
            total_orders: 10,
            verified_orders: 10,
            chain_intact: false,
            chain_resets: vec![ChainReset {
                receipt_number: "FAC2026012910005".to_string(),
                prev_chain_hash: "ddd".to_string(),
            }],
            chain_breaks: vec![ChainBreak {
                receipt_number: "FAC2026012910008".to_string(),
                expected_prev_hash: "ggg".to_string(),
                actual_prev_hash: "tampered".to_string(),
            }],
            invalid_orders: vec![],
        };
        assert!(!verification.chain_intact);
        assert_eq!(verification.chain_resets.len(), 1);
        assert_eq!(verification.chain_breaks.len(), 1);
    }

    #[test]
    fn test_order_verification_model() {
        let verification = OrderVerification {
            receipt_number: "FAC2026012910001".to_string(),
            order_id: "order:abc123".to_string(),
            prev_hash: "genesis".to_string(),
            curr_hash: "some_hash".to_string(),
            events_chain_valid: true,
            event_count: 5,
            invalid_events: vec![],
        };
        assert!(verification.events_chain_valid);
        assert_eq!(verification.event_count, 5);

        let json = serde_json::to_string(&verification).unwrap();
        assert!(json.contains("\"events_chain_valid\":true"));
    }

    #[test]
    fn test_chain_break_model() {
        let brk = ChainBreak {
            receipt_number: "FAC2026012910003".to_string(),
            expected_prev_hash: "hash_of_order_2".to_string(),
            actual_prev_hash: "tampered_hash".to_string(),
        };

        let json = serde_json::to_string(&brk).unwrap();
        assert!(json.contains("\"expected_prev_hash\":\"hash_of_order_2\""));
        assert!(json.contains("\"actual_prev_hash\":\"tampered_hash\""));
    }
}
