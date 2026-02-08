//! TableOpened event applier
//!
//! Applies the TableOpened event to create initial snapshot state.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot, OrderStatus};

/// TableOpened applier
pub struct TableOpenedApplier;

impl EventApplier for TableOpenedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::TableOpened {
            table_id,
            table_name,
            zone_id,
            zone_name,
            guest_count,
            is_retail,
            queue_number,
            receipt_number,
        } = &event.payload
        {
            // Set order_id from event (important for replay scenarios)
            snapshot.order_id = event.order_id.clone();
            snapshot.table_id = *table_id;
            snapshot.table_name = table_name.clone();
            snapshot.zone_id = *zone_id;
            snapshot.zone_name = zone_name.clone();
            snapshot.guest_count = *guest_count;
            snapshot.is_retail = *is_retail;
            snapshot.queue_number = *queue_number;
            snapshot.receipt_number = receipt_number.clone();
            snapshot.status = OrderStatus::Active;
            snapshot.start_time = event.timestamp;
            snapshot.created_at = event.timestamp;
            snapshot.updated_at = event.timestamp;
            snapshot.last_sequence = event.sequence;

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_opened_applier() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());

        let event = OrderEvent::new(
            1,
            "order-1".to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            shared::order::OrderEventType::TableOpened,
            EventPayload::TableOpened {
                table_id: Some(1),
                table_name: Some("Table 1".to_string()),
                zone_id: Some(1),
                zone_name: Some("Zone 1".to_string()),
                guest_count: 4,
                is_retail: false,
                queue_number: None,
                receipt_number: "RCP-TEST-001".to_string(),
            },
        );

        let applier = TableOpenedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.table_id, Some(1));
        assert_eq!(snapshot.table_name, Some("Table 1".to_string()));
        assert_eq!(snapshot.guest_count, 4);
        assert_eq!(snapshot.status, OrderStatus::Active);
        assert_eq!(snapshot.last_sequence, 1);
    }
}
