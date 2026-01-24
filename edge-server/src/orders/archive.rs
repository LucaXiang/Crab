//! Order Archiving Service
//!
//! Archives completed orders from redb to SurrealDB with hash chain integrity.

use crate::db::models::{
    Order as SurrealOrder, OrderEventType as SurrealEventType, OrderStatus as SurrealOrderStatus,
};
use crate::db::repository::{OrderRepository, SystemStateRepository};
use sha2::{Digest, Sha256};
use shared::order::{OrderEvent, OrderEventType, OrderSnapshot, OrderStatus};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use thiserror::Error;

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

/// Service for archiving orders to SurrealDB
#[derive(Clone)]
pub struct OrderArchiveService {
    order_repo: OrderRepository,
    system_state_repo: SystemStateRepository,
}

impl OrderArchiveService {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            order_repo: OrderRepository::new(db.clone()),
            system_state_repo: SystemStateRepository::new(db),
        }
    }

    /// Archive a completed order with its events
    pub async fn archive_order(
        &self,
        snapshot: &OrderSnapshot,
        events: Vec<OrderEvent>,
    ) -> ArchiveResult<()> {
        // 1. Get last order hash from system_state
        let system_state = self
            .system_state_repo
            .get_or_create()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let prev_hash = system_state
            .last_order_hash
            .unwrap_or_else(|| "genesis".to_string());

        // 2. Compute order hash (includes last event hash)
        let last_event_hash = events
            .last()
            .map(|e| self.compute_event_hash(e))
            .unwrap_or_else(|| "no_events".to_string());

        let order_hash = self.compute_order_hash(snapshot, &prev_hash, &last_event_hash);

        // 3. Convert and store order
        let surreal_order =
            self.convert_snapshot_to_order(snapshot, prev_hash, order_hash.clone())?;
        let created_order = self
            .order_repo
            .create_archived(surreal_order)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let order_id = created_order
            .id
            .ok_or_else(|| ArchiveError::Database("Order has no ID".to_string()))?;

        // 4. Store events with RELATE
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

        // 5. Update system_state with new last_order_hash
        self.system_state_repo
            .update_last_order(&order_id.to_string(), order_hash)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(order_id = %snapshot.order_id, "Order archived to SurrealDB");
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

    fn compute_event_hash(&self, event: &OrderEvent) -> String {
        let mut hasher = Sha256::new();
        hasher.update(event.event_id.as_bytes());
        hasher.update(event.order_id.as_bytes());
        hasher.update(format!("{}", event.sequence).as_bytes());
        hasher.update(format!("{:?}", event.event_type).as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn convert_snapshot_to_order(
        &self,
        snapshot: &OrderSnapshot,
        prev_hash: String,
        curr_hash: String,
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
            start_time: chrono::DateTime::from_timestamp_millis(snapshot.start_time)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
            end_time: snapshot.end_time.map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            }),
            guest_count: Some(snapshot.guest_count),
            total_amount: snapshot.total,
            paid_amount: snapshot.paid_amount,
            discount_amount: snapshot.total_discount,
            surcharge_amount: snapshot.total_surcharge,
            items: vec![], // Items embedded in events
            payments: vec![], // Payments embedded in events
            prev_hash,
            curr_hash,
            related_order_id: None,
            operator_id: None,
            created_at: None,
        })
    }

    fn convert_event_type(&self, event_type: &OrderEventType) -> SurrealEventType {
        match event_type {
            OrderEventType::TableOpened => SurrealEventType::Created,
            OrderEventType::ItemsAdded => SurrealEventType::ItemAdded,
            OrderEventType::ItemRemoved => SurrealEventType::ItemRemoved,
            OrderEventType::ItemModified => SurrealEventType::ItemUpdated,
            OrderEventType::PaymentAdded => SurrealEventType::PartialPaid,
            OrderEventType::OrderCompleted => SurrealEventType::Paid,
            OrderEventType::OrderVoided => SurrealEventType::Void,
            // Fallback for other event types
            _ => SurrealEventType::ItemUpdated,
        }
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
        // Hash should be deterministic for same inputs
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

    // Standalone functions for testing without OrderArchiveService
    fn compute_order_hash_standalone(snapshot: &OrderSnapshot, prev_hash: &str, last_event_hash: &str) -> String {
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
        format!("{:x}", hasher.finalize())
    }
}
