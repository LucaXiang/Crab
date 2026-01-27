//! Order Archiving Service
//!
//! Archives completed orders from redb to SurrealDB with hash chain integrity.
//! Uses graph model with RELATE edges for items, options, payments, events.
//! All archive operations are atomic - either everything succeeds or nothing is written.

use crate::db::models::{
    Order as SurrealOrder, OrderStatus as SurrealOrderStatus, SplitItem,
};
use crate::db::repository::SystemStateRepository;
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
        let receipt = snapshot.receipt_number.clone().unwrap_or_default();
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
        let surreal_order =
            self.convert_snapshot_to_order(snapshot, prev_hash, order_hash.clone(), operator_id, operator_name)?;

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
        hasher.update(
            snapshot
                .receipt_number
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
        );
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
            receipt_number: snapshot.receipt_number.clone().unwrap_or_default(),
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
            receipt_number: Some("R001".to_string()),
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
                receipt_number: None,
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
            receipt_number: None,
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
        hasher.update(snapshot.receipt_number.as_deref().unwrap_or("").as_bytes());
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
}
