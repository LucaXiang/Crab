//! Order Archiving Service
//!
//! Archives completed orders from redb to SQLite with hash chain integrity.
//! Uses relational model with foreign keys for items, options, payments, events.
//! All archive operations are atomic - either everything succeeds or nothing is written.

use crate::db::repository::system_state;
use super::money::{to_decimal, to_f64};
use rust_decimal::Decimal;
use serde::Serialize;
use sha2::{Digest, Sha256};
use shared::order::{OrderEvent, OrderEventType, OrderSnapshot, OrderStatus};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
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

impl From<ArchiveError> for shared::error::AppError {
    fn from(err: ArchiveError) -> Self {
        use shared::error::AppError;
        match err {
            ArchiveError::Database(msg) => AppError::database(msg),
            ArchiveError::HashChain(msg) => AppError::internal(msg),
            ArchiveError::Conversion(msg) => AppError::internal(msg),
        }
    }
}

impl From<sqlx::Error> for ArchiveError {
    fn from(err: sqlx::Error) -> Self {
        ArchiveError::Database(err.to_string())
    }
}

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

#[derive(Debug, sqlx::FromRow)]
struct VerifyOrderRow {
    id: i64,
    receipt_number: String,
    prev_hash: String,
    curr_hash: String,
}

#[derive(Debug, sqlx::FromRow)]
struct VerifyEventRow {
    id: i64,
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

/// Service for archiving orders to SQLite
#[derive(Clone)]
pub struct OrderArchiveService {
    pool: SqlitePool,
    /// Semaphore to limit concurrent archive tasks
    archive_semaphore: Arc<Semaphore>,
    /// Mutex to ensure hash chain updates are serialized (prevents race conditions)
    hash_chain_lock: Arc<Mutex<()>>,
    /// Directory for storing failed archives
    bad_archive_dir: PathBuf,
    /// 业务时区 (用于收据编号日期)
    tz: chrono_tz::Tz,
}

impl OrderArchiveService {
    pub fn new(pool: SqlitePool, tz: chrono_tz::Tz) -> Self {
        let bad_archive_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("data")
            .join("bad_archives");

        Self {
            pool,
            archive_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_ARCHIVES)),
            hash_chain_lock: Arc::new(Mutex::new(())),
            bad_archive_dir,
            tz,
        }
    }

    /// Generate the next receipt number atomically
    ///
    /// Format: FAC{YYYYMMDD}{sequence}
    /// Example: FAC2026012410001
    pub async fn generate_next_receipt_number(&self) -> ArchiveResult<String> {
        let next_num = system_state::get_next_order_number(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let now = chrono::Utc::now().with_timezone(&self.tz);
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
        let _permit = self
            .archive_semaphore
            .acquire()
            .await
            .map_err(|_| ArchiveError::Database("Archive semaphore closed".to_string()))?;

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
        let error =
            last_error.unwrap_or_else(|| ArchiveError::Database("Unknown error".to_string()));
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
            timestamp: i64,
        }

        let bad_archive = BadArchive {
            snapshot: snapshot.clone(),
            events: events.to_vec(),
            error: error.to_string(),
            timestamp: shared::util::now_millis(),
        };

        // Create directory if needed
        if let Err(e) = std::fs::create_dir_all(&self.bad_archive_dir) {
            tracing::error!(error = %e, "Failed to create bad archive directory");
            return;
        }

        let filename = format!("{}-{}.json", shared::util::now_millis(), snapshot.order_id);
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
        // 0. Idempotency check
        let exists: bool = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM archived_order WHERE receipt_number = ?)",
        )
        .bind(&snapshot.receipt_number)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        if exists {
            tracing::info!(order_id = %snapshot.order_id, "Order already archived, skipping");
            return Ok(());
        }

        // Acquire hash chain lock to prevent concurrent hash chain corruption
        let _hash_lock = self.hash_chain_lock.lock().await;

        // 1. Get last order hash from system_state
        let system_state = system_state::get_or_create(&self.pool)
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
            .map(|e| (Some(e.operator_id), Some(e.operator_name.clone())))
            .unwrap_or((None, None));

        // 4. Status string
        let status_str = match snapshot.status {
            OrderStatus::Completed => "COMPLETED",
            OrderStatus::Void => "VOID",
            OrderStatus::Merged => "MERGED",
            _ => {
                return Err(ArchiveError::Conversion(format!(
                    "Cannot archive order with status {:?}",
                    snapshot.status
                )));
            }
        };

        let now = shared::util::now_millis();

        tracing::debug!(
            order_id = %snapshot.order_id,
            items_count = snapshot.items.len(),
            payments_count = snapshot.payments.len(),
            events_count = events.len(),
            "Archiving order (atomic transaction)"
        );

        // 5. Begin SQLite transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 5a. INSERT archived_order
        let order_pk = sqlx::query_scalar::<_, i64>(
            "INSERT INTO archived_order (\
                receipt_number, zone_name, table_name, status, is_retail, guest_count, \
                original_total, subtotal, total_amount, paid_amount, \
                discount_amount, surcharge_amount, comp_total_amount, \
                order_manual_discount_amount, order_manual_surcharge_amount, \
                order_rule_discount_amount, order_rule_surcharge_amount, \
                tax, start_time, end_time, \
                operator_id, operator_name, \
                void_type, loss_reason, loss_amount, void_note, \
                member_id, member_name, \
                prev_hash, curr_hash, created_at\
            ) VALUES (\
                ?1, ?2, ?3, ?4, ?5, ?6, \
                ?7, ?8, ?9, ?10, \
                ?11, ?12, ?13, \
                ?14, ?15, \
                ?16, ?17, \
                ?18, ?19, ?20, \
                ?21, ?22, \
                ?23, ?24, ?25, ?26, \
                ?27, ?28, \
                ?29, ?30, ?31\
            ) RETURNING id",
        )
        .bind(&snapshot.receipt_number)
        .bind(&snapshot.zone_name)
        .bind(&snapshot.table_name)
        .bind(status_str)
        .bind(snapshot.is_retail)
        .bind(snapshot.guest_count)
        .bind(snapshot.original_total)
        .bind(snapshot.subtotal)
        .bind(snapshot.total)
        .bind(snapshot.paid_amount)
        .bind(snapshot.total_discount)
        .bind(snapshot.total_surcharge)
        .bind(snapshot.comp_total_amount)
        .bind(snapshot.order_manual_discount_amount)
        .bind(snapshot.order_manual_surcharge_amount)
        .bind(snapshot.order_rule_discount_amount)
        .bind(snapshot.order_rule_surcharge_amount)
        .bind(snapshot.tax)
        .bind(snapshot.start_time)
        .bind(snapshot.end_time)
        .bind(operator_id)
        .bind(&operator_name)
        .bind(snapshot.void_type.as_ref().map(|v| {
            serde_json::to_value(v)
                .ok()
                .and_then(|val| val.as_str().map(String::from))
                .unwrap_or_default()
        }))
        .bind(snapshot.loss_reason.as_ref().map(|r| {
            serde_json::to_value(r)
                .ok()
                .and_then(|val| val.as_str().map(String::from))
                .unwrap_or_default()
        }))
        .bind(snapshot.loss_amount)
        .bind(&snapshot.void_note)
        .bind(snapshot.member_id)
        .bind(&snapshot.member_name)
        .bind(&prev_hash)
        .bind(&order_hash)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 5b. INSERT items and their options
        for item in &snapshot.items {
            // Compute item prices using Decimal
            let base_price = if item.original_price > 0.0 { item.original_price } else { item.price };
            let d_base = to_decimal(base_price);
            let d_qty = Decimal::from(item.quantity);
            let d_manual_discount_per_unit = item
                .manual_discount_percent
                .map(|p| d_base * to_decimal(p) / Decimal::ONE_HUNDRED)
                .unwrap_or(Decimal::ZERO);
            let d_rule_discount_per_unit = to_decimal(item.rule_discount_amount);
            let total_discount =
                to_f64((d_manual_discount_per_unit + d_rule_discount_per_unit) * d_qty);
            let d_surcharge_per_unit = to_decimal(item.rule_surcharge_amount);
            let total_surcharge = to_f64(d_surcharge_per_unit * d_qty);
            let unit_price = if item.unit_price > 0.0 || item.is_comped {
                item.unit_price
            } else {
                to_f64(
                    d_base - d_manual_discount_per_unit - d_rule_discount_per_unit
                        + d_surcharge_per_unit,
                )
            };
            let line_total = if item.line_total > 0.0 || item.is_comped {
                item.line_total
            } else {
                to_f64(to_decimal(unit_price) * d_qty)
            };
            let spec_name = item.selected_specification.as_ref().map(|s| s.name.clone());
            let instance_id = item.instance_id.clone();
            let paid_qty = snapshot
                .paid_item_quantities
                .get(&instance_id)
                .copied()
                .unwrap_or(0);
            let unpaid_quantity = (item.quantity - paid_qty).max(0);
            let rule_discount_total = to_f64(d_rule_discount_per_unit * d_qty);
            let rule_surcharge_total = to_f64(d_surcharge_per_unit * d_qty);

            let item_pk = sqlx::query_scalar::<_, i64>(
                "INSERT INTO archived_order_item (\
                    order_pk, spec, instance_id, name, spec_name, price, \
                    quantity, unpaid_quantity, unit_price, line_total, \
                    discount_amount, surcharge_amount, \
                    rule_discount_amount, rule_surcharge_amount, \
                    tax, tax_rate, category_id, category_name, applied_rules, note, is_comped\
                ) VALUES (\
                    ?1, ?2, ?3, ?4, ?5, ?6, \
                    ?7, ?8, ?9, ?10, \
                    ?11, ?12, \
                    ?13, ?14, \
                    ?15, ?16, ?17, ?18, ?19, ?20, ?21\
                ) RETURNING id",
            )
            .bind(order_pk)
            .bind(item.id)
            .bind(&instance_id)
            .bind(&item.name)
            .bind(&spec_name)
            .bind(base_price)
            .bind(item.quantity)
            .bind(unpaid_quantity)
            .bind(unit_price)
            .bind(line_total)
            .bind(total_discount)
            .bind(total_surcharge)
            .bind(rule_discount_total)
            .bind(rule_surcharge_total)
            .bind(item.tax)
            .bind(item.tax_rate)
            .bind(item.category_id)
            .bind(&item.category_name)
            .bind(
                if item.applied_rules.is_empty() {
                    None
                } else {
                    serde_json::to_string(&item.applied_rules).ok()
                },
            )
            .bind(&item.note)
            .bind(item.is_comped)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

            // Options
            if let Some(options) = &item.selected_options {
                for opt in options {
                    let price = opt.price_modifier.unwrap_or(0.0);
                    let opt_qty = opt.quantity;
                    sqlx::query!(
                        "INSERT INTO archived_order_item_option (\
                            item_pk, attribute_name, option_name, price, quantity\
                        ) VALUES (?1, ?2, ?3, ?4, ?5)",
                        item_pk,
                        opt.attribute_name,
                        opt.option_name,
                        price,
                        opt_qty,
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ArchiveError::Database(e.to_string()))?;
                }
            }
        }

        // 5c. INSERT payments
        for (i, payment) in snapshot.payments.iter().enumerate() {
            // Serialize split_items to JSON string
            let split_items_str: Option<String> = payment.split_items.as_ref().map(|items| {
                #[derive(serde::Serialize)]
                struct ArchiveSplitItem {
                    instance_id: String,
                    name: String,
                    quantity: i32,
                    unit_price: f64,
                }
                let archive_items: Vec<ArchiveSplitItem> = items
                    .iter()
                    .map(|si| ArchiveSplitItem {
                        instance_id: si.instance_id.clone(),
                        name: si.name.clone(),
                        quantity: si.quantity,
                        unit_price: if si.unit_price > 0.0 { si.unit_price } else { si.price },
                    })
                    .collect();
                serde_json::to_string(&archive_items).unwrap_or_else(|_| "[]".to_string())
            });

            let seq = i as i32;
            let split_type_str = payment.split_type.as_ref().map(|st| {
                serde_json::to_value(st)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default()
            });

            sqlx::query!(
                "INSERT INTO archived_order_payment (\
                    order_pk, seq, payment_id, method, amount, time, \
                    cancelled, cancel_reason, \
                    tendered, change_amount, \
                    split_type, split_items, aa_shares, aa_total_shares\
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                order_pk,
                seq,
                payment.payment_id,
                payment.method,
                payment.amount,
                payment.timestamp,
                payment.cancelled,
                payment.cancel_reason,
                payment.tendered,
                payment.change,
                split_type_str,
                split_items_str,
                payment.aa_shares,
                snapshot.aa_total_shares,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        // 5d. INSERT events
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

            // Serialize payload to JSON string
            let payload_str =
                serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".to_string());

            let seq = i as i32;

            sqlx::query!(
                "INSERT INTO archived_order_event (\
                    order_pk, seq, event_type, timestamp, data, prev_hash, curr_hash\
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                order_pk,
                seq,
                event_type_str,
                event.timestamp,
                payload_str,
                prev_event_hash,
                curr_event_hash,
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        // 5e. Update system_state
        sqlx::query!(
            "UPDATE system_state SET last_order_hash = ?1, updated_at = ?2 WHERE id = 1",
            order_hash,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(order_id = %snapshot.order_id, "Order archived successfully");
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
        hasher.update(b"\x00");
        hasher.update(snapshot.order_id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(snapshot.receipt_number.as_bytes());
        hasher.update(b"\x00");
        // SAFETY: OrderStatus derives Serialize and always succeeds
        let status_str = serde_json::to_string(&snapshot.status)
            .expect("OrderStatus serialization is infallible");
        hasher.update(status_str.as_bytes());
        hasher.update(b"\x00");
        hasher.update(last_event_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute event hash including payload for tamper-proofing
    fn compute_event_hash(&self, event: &OrderEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event.event_id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(event.order_id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(event.sequence.to_le_bytes());
        // SAFETY: OrderEventType derives Serialize and always succeeds
        let event_type_str = serde_json::to_string(&event.event_type)
            .expect("OrderEventType serialization is infallible");
        hasher.update(event_type_str.as_bytes());
        hasher.update(b"\x00");
        // SAFETY: EventPayload derives Serialize and always succeeds
        let payload_json = serde_json::to_string(&event.payload)
            .expect("EventPayload serialization is infallible");
        hasher.update(payload_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    // ========================================================================
    // Verification Methods
    // ========================================================================

    /// 验证单个订单的事件哈希链完整性
    pub async fn verify_order(&self, receipt_number: &str) -> ArchiveResult<OrderVerification> {
        // 1. 查询订单
        let order: VerifyOrderRow = sqlx::query_as::<_, VerifyOrderRow>(
            "SELECT id, receipt_number, prev_hash, curr_hash \
             FROM archived_order WHERE receipt_number = ?",
        )
        .bind(receipt_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| ArchiveError::Database(format!("Order not found: {}", receipt_number)))?;

        // 2. 查询该订单的所有事件（按 seq 排序）
        let events: Vec<VerifyEventRow> = sqlx::query_as::<_, VerifyEventRow>(
            "SELECT id, event_type, prev_hash, curr_hash \
             FROM archived_order_event WHERE order_pk = ? ORDER BY seq",
        )
        .bind(order.id)
        .fetch_all(&self.pool)
        .await
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
                    event_id: event.id.to_string(),
                    event_type: event.event_type.clone(),
                    expected_prev_hash: expected_prev,
                    actual_prev_hash: event.prev_hash.clone(),
                    valid: false,
                });
            }
        }

        Ok(OrderVerification {
            receipt_number: order.receipt_number,
            order_id: order.id.to_string(),
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
    /// - `start`/`end`: Unix millis 时间范围（由 handler 层根据 business_day_cutoff 计算）
    pub async fn verify_daily_chain(
        &self,
        date: &str,
        start: i64,
        end: i64,
    ) -> ArchiveResult<DailyChainVerification> {
        // 1. 查询当天所有订单，按 created_at 排序
        let orders: Vec<VerifyOrderRow> = sqlx::query_as::<_, VerifyOrderRow>(
            "SELECT id, receipt_number, prev_hash, curr_hash \
             FROM archived_order \
             WHERE created_at >= ?1 AND created_at < ?2 \
             ORDER BY created_at",
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let total_orders = orders.len();

        // 2. 查询范围之前最后一个订单的 curr_hash
        let prev_day_hash: Option<String> = sqlx::query_scalar::<_, String>(
            "SELECT curr_hash FROM archived_order \
             WHERE created_at < ? \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(start)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 确定当天第一个订单的 expected prev_hash
        // 有前一天订单 -> 用其 curr_hash；没有 -> 用第一个订单自身的 prev_hash（不猜）
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{OrderEventType, OrderSnapshot, OrderStatus};

    fn create_test_snapshot() -> OrderSnapshot {
        OrderSnapshot {
            order_id: "test-order-1".to_string(),
            table_id: Some(1),
            table_name: Some("Table 1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            service_type: None,
            queue_number: None,
            status: OrderStatus::Completed,
            void_type: None,
            loss_reason: None,
            loss_amount: None,
            void_note: None,
            items: vec![],
            comps: vec![],
            payments: vec![],
            original_total: 100.0,
            subtotal: 100.0,
            total_discount: 0.0,
            total_surcharge: 0.0,
            tax: 0.0,
            discount: 0.0,
            comp_total_amount: 0.0,
            order_manual_discount_amount: 0.0,
            order_manual_surcharge_amount: 0.0,
            total: 100.0,
            paid_amount: 100.0,
            remaining_amount: 0.0,
            paid_item_quantities: std::collections::BTreeMap::new(),
            receipt_number: "R001".to_string(),
            is_pre_payment: false,
            note: None,
            order_rule_discount_amount: 0.0,
            order_rule_surcharge_amount: 0.0,
            order_applied_rules: vec![],
            order_manual_discount_percent: None,
            order_manual_discount_fixed: None,
            order_manual_surcharge_percent: None,
            order_manual_surcharge_fixed: None,
            member_id: None,
            member_name: None,
            marketing_group_id: None,
            marketing_group_name: None,
            mg_discount_amount: 0.0,
            stamp_redemptions: Vec::new(),
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
            operator_id: 1,
            operator_name: "Test Operator".to_string(),
            command_id: format!("cmd-{}", sequence),
            event_type: OrderEventType::TableOpened,
            payload: shared::order::EventPayload::TableOpened {
                table_id: Some(1),
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
            table_id: Some(2), // Different table
            table_name: Some("Table 2".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            queue_number: None,
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
        hasher.update(b"\x00");
        hasher.update(snapshot.order_id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(snapshot.receipt_number.as_bytes());
        hasher.update(b"\x00");
        let status_str = serde_json::to_string(&snapshot.status).unwrap_or_default();
        hasher.update(status_str.as_bytes());
        hasher.update(b"\x00");
        hasher.update(last_event_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn compute_event_hash_standalone(event: &shared::order::OrderEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event.event_id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(event.order_id.as_bytes());
        hasher.update(b"\x00");
        hasher.update(event.sequence.to_le_bytes());
        let event_type_str = serde_json::to_string(&event.event_type).unwrap_or_default();
        hasher.update(event_type_str.as_bytes());
        hasher.update(b"\x00");
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
                id: i as i64,
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
                    event_id: event.id.to_string(),
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
        assert_eq!(invalid[0].event_id, "0");
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
        assert_eq!(invalid[0].event_id, "2");
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
        assert_eq!(invalid[0].event_id, "1");
        assert_eq!(invalid[1].event_id, "3");
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
                id: i as i64,
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
        // Validate with wrong expected prev -- simulates missing previous day data
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
