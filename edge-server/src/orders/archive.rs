//! Order Archiving Service
//!
//! Archives completed orders from redb to SurrealDB with hash chain integrity.
//! Uses graph model with RELATE edges for items, options, payments, events.

use crate::db::models::{
    Order as SurrealOrder, OrderEventType as SurrealEventType, OrderStatus as SurrealOrderStatus,
    SplitItem,
};
use crate::db::repository::{OrderRepository, SystemStateRepository};
use sha2::{Digest, Sha256};
use shared::order::{OrderEvent, OrderEventType, OrderSnapshot, OrderStatus};
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::RecordId;
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
    order_repo: OrderRepository,
    system_state_repo: SystemStateRepository,
    /// Semaphore to limit concurrent archive tasks
    archive_semaphore: Arc<Semaphore>,
    /// Mutex to ensure hash chain updates are serialized (prevents race conditions)
    hash_chain_lock: Arc<Mutex<()>>,
}

impl OrderArchiveService {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            db: db.clone(),
            order_repo: OrderRepository::new(db.clone()),
            system_state_repo: SystemStateRepository::new(db),
            archive_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_ARCHIVES)),
            hash_chain_lock: Arc::new(Mutex::new(())),
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

        Err(last_error.unwrap_or_else(|| ArchiveError::Database("Unknown error".to_string())))
    }

    /// Internal archive implementation (single attempt)
    async fn archive_order_internal(
        &self,
        snapshot: &OrderSnapshot,
        events: &[OrderEvent],
    ) -> ArchiveResult<()> {
        // 0. Check idempotency - skip if already archived
        if let Ok(true) = self
            .order_repo
            .exists_by_receipt(snapshot.receipt_number.as_deref().unwrap_or(""))
            .await
        {
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

        // 4. Convert and store order
        let surreal_order =
            self.convert_snapshot_to_order(snapshot, prev_hash, order_hash.clone(), operator_id, operator_name)?;

        tracing::info!(
            order_id = %snapshot.order_id,
            items_count = snapshot.items.len(),
            "Archiving order to SurrealDB (graph model)"
        );

        let created_order = self
            .order_repo
            .create_archived(surreal_order)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let order_id = created_order
            .id
            .ok_or_else(|| ArchiveError::Database("Order has no ID".to_string()))?;

        // 5. Store items with RELATE has_item edges
        self.store_items_with_edges(&order_id, snapshot).await?;

        // 6. Store payments with RELATE has_payment edges
        self.store_payments_with_edges(&order_id, snapshot).await?;

        // 7. Store events with RELATE has_event edges
        for (i, event) in events.iter().enumerate() {
            let prev_event_hash = if i == 0 {
                "order_start".to_string()
            } else {
                self.compute_event_hash(&events[i - 1])
            };
            let curr_event_hash = self.compute_event_hash(event);

            self.order_repo
                .add_event(
                    &order_id.key().to_string(),
                    self.convert_event_type(&event.event_type),
                    Some(serde_json::to_value(&event.payload).unwrap()),
                    prev_event_hash,
                    curr_event_hash,
                )
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        // 8. Update system_state with new last_order_hash
        self.system_state_repo
            .update_last_order(&order_id.to_string(), order_hash)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(order_id = %snapshot.order_id, "Order archived to SurrealDB (graph model)");
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
            total_amount: snapshot.total,
            paid_amount: snapshot.paid_amount,
            discount_amount: snapshot.total_discount,
            surcharge_amount: snapshot.total_surcharge,
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

    fn convert_event_type(&self, event_type: &OrderEventType) -> SurrealEventType {
        match event_type {
            // Lifecycle
            OrderEventType::TableOpened => SurrealEventType::TableOpened,
            OrderEventType::OrderCompleted => SurrealEventType::OrderCompleted,
            OrderEventType::OrderVoided => SurrealEventType::OrderVoided,
            OrderEventType::OrderRestored => SurrealEventType::OrderRestored,
            // Items
            OrderEventType::ItemsAdded => SurrealEventType::ItemsAdded,
            OrderEventType::ItemModified => SurrealEventType::ItemModified,
            OrderEventType::ItemRemoved => SurrealEventType::ItemRemoved,
            OrderEventType::ItemRestored => SurrealEventType::ItemRestored,
            // Payments
            OrderEventType::PaymentAdded => SurrealEventType::PaymentAdded,
            OrderEventType::PaymentCancelled => SurrealEventType::PaymentCancelled,
            // Split
            OrderEventType::OrderSplit => SurrealEventType::OrderSplit,
            // Table operations
            OrderEventType::OrderMoved => SurrealEventType::OrderMoved,
            OrderEventType::OrderMovedOut => SurrealEventType::OrderMovedOut,
            OrderEventType::OrderMerged => SurrealEventType::OrderMerged,
            OrderEventType::OrderMergedOut => SurrealEventType::OrderMergedOut,
            OrderEventType::TableReassigned => SurrealEventType::TableReassigned,
            // Other
            OrderEventType::OrderInfoUpdated => SurrealEventType::OrderInfoUpdated,
            // Price Rules
            OrderEventType::RuleSkipToggled => SurrealEventType::RuleSkipToggled,
        }
    }

    /// Store order items with RELATE has_item edges
    async fn store_items_with_edges(
        &self,
        order_id: &RecordId,
        snapshot: &OrderSnapshot,
    ) -> ArchiveResult<()> {
        for item in &snapshot.items {
            // Use original_price as base for discount calculation
            let base_price = item.original_price.unwrap_or(item.price);

            // Calculate per-unit discount amounts
            let manual_discount_per_unit = item
                .manual_discount_percent
                .map(|p| base_price * p / 100.0)
                .unwrap_or(0.0);
            let rule_discount_per_unit = item.rule_discount_amount.unwrap_or(0.0);

            // Total discount = per-unit discount * quantity
            let total_discount =
                (manual_discount_per_unit + rule_discount_per_unit) * item.quantity as f64;

            // Total surcharge = per-unit surcharge * quantity
            let surcharge_per_unit =
                item.surcharge.unwrap_or(0.0) + item.rule_surcharge_amount.unwrap_or(0.0);
            let total_surcharge = surcharge_per_unit * item.quantity as f64;

            // Use pre-calculated values from snapshot
            let unit_price = item.unit_price.unwrap_or_else(|| {
                base_price - manual_discount_per_unit - rule_discount_per_unit + surcharge_per_unit
            });
            let line_total = item
                .line_total
                .unwrap_or_else(|| unit_price * item.quantity as f64);

            // Get spec_name from selected specification
            let spec_name = item
                .selected_specification
                .as_ref()
                .map(|s| s.name.clone());

            // Get instance_id (content-addressable hash)
            let instance_id = item.instance_id.clone();

            // Calculate unpaid_quantity
            let paid_qty = snapshot.paid_item_quantities.get(&instance_id).copied().unwrap_or(0);
            let unpaid_quantity = (item.quantity - paid_qty).max(0);

            // Create order_item and RELATE to order
            let mut result = self
                .db
                .query(
                    r#"
                    LET $item = (CREATE order_item SET
                        spec = $spec,
                        instance_id = $instance_id,
                        name = $name,
                        spec_name = $spec_name,
                        price = $price,
                        quantity = $quantity,
                        unpaid_quantity = $unpaid_quantity,
                        unit_price = $unit_price,
                        line_total = $line_total,
                        discount_amount = $discount_amount,
                        surcharge_amount = $surcharge_amount,
                        note = $note
                    );
                    RELATE $order_id->has_item->$item;
                    RETURN $item.id;
                "#,
                )
                .bind(("order_id", order_id.clone()))
                .bind(("spec", item.id.clone()))
                .bind(("instance_id", instance_id.clone()))
                .bind(("name", item.name.clone()))
                .bind(("spec_name", spec_name))
                .bind(("price", base_price))
                .bind(("quantity", item.quantity))
                .bind(("unpaid_quantity", unpaid_quantity))
                .bind(("unit_price", unit_price))
                .bind(("line_total", line_total))
                .bind(("discount_amount", total_discount))
                .bind(("surcharge_amount", total_surcharge))
                .bind(("note", item.note.clone()))
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;

            // Get created item ID for storing options
            let item_ids: Vec<RecordId> = result
                .take(0)
                .map_err(|e| ArchiveError::Database(e.to_string()))?;

            let item_id = item_ids
                .into_iter()
                .next()
                .ok_or_else(|| ArchiveError::Database("Failed to create order_item".to_string()))?;

            // Store options with RELATE has_option edges
            if let Some(options) = &item.selected_options {
                for opt in options {
                    self.db
                        .query(
                            r#"
                            LET $opt = (CREATE order_item_option SET
                                attribute_name = $attribute_name,
                                option_name = $option_name,
                                price = $price
                            );
                            RELATE $item_id->has_option->$opt;
                        "#,
                        )
                        .bind(("item_id", item_id.clone()))
                        .bind(("attribute_name", opt.attribute_name.clone()))
                        .bind(("option_name", opt.option_name.clone()))
                        .bind(("price", opt.price_modifier.unwrap_or(0.0)))
                        .await
                        .map_err(|e| ArchiveError::Database(e.to_string()))?;
                }
            }
        }

        Ok(())
    }

    /// Store payments with RELATE has_payment edges
    async fn store_payments_with_edges(
        &self,
        order_id: &RecordId,
        snapshot: &OrderSnapshot,
    ) -> ArchiveResult<()> {
        for payment in &snapshot.payments {
            let time = chrono::DateTime::from_timestamp_millis(payment.timestamp)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();

            // Build split_items from payment if available
            let split_items: Vec<SplitItem> = payment
                .split_items
                .as_ref()
                .map(|items| {
                    items
                        .iter()
                        .map(|si| SplitItem {
                            instance_id: si.instance_id.clone(),
                            name: si.name.clone(),
                            quantity: si.quantity,
                        })
                        .collect()
                })
                .unwrap_or_default();

            self.db
                .query(
                    r#"
                    LET $payment = (CREATE order_payment SET
                        method = $method,
                        amount = $amount,
                        time = <datetime>$time,
                        reference = $reference,
                        cancelled = $cancelled,
                        cancel_reason = $cancel_reason,
                        split_items = $split_items
                    );
                    RELATE $order_id->has_payment->$payment;
                "#,
                )
                .bind(("order_id", order_id.clone()))
                .bind(("method", payment.method.clone()))
                .bind(("amount", payment.amount))
                .bind(("time", time))
                .bind(("reference", payment.note.clone()))
                .bind(("cancelled", payment.cancelled))
                .bind(("cancel_reason", payment.cancel_reason.clone()))
                .bind(("split_items", split_items))
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        Ok(())
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
        let mut event1 = create_test_event("order-1", 1);
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
