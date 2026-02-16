//! OrderNoteAdded event applier
//!
//! Applies the OrderNoteAdded event to set or clear the order note.
//! Empty note string clears the note (sets to None).
//! Does NOT affect financial calculations.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// OrderNoteAdded applier
pub struct OrderNoteAddedApplier;

impl EventApplier for OrderNoteAddedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::OrderNoteAdded { note, .. } = &event.payload {
            // Set note: empty string = clear (None), otherwise Some
            snapshot.note = if note.is_empty() {
                None
            } else {
                Some(note.clone())
            };

            // Update sequence and timestamp
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Update checksum (no recalculate_totals needed - note doesn't affect money)
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{OrderEventType, OrderStatus};

    fn create_test_snapshot(order_id: &str) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id.to_string());
        snapshot.status = OrderStatus::Active;
        snapshot
    }

    fn create_order_note_added_event(
        order_id: &str,
        seq: u64,
        note: &str,
        previous_note: Option<String>,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderNoteAdded,
            EventPayload::OrderNoteAdded {
                note: note.to_string(),
                previous_note,
            },
        )
    }

    #[test]
    fn test_apply_note_sets_snapshot_note() {
        let mut snapshot = create_test_snapshot("order-1");
        assert_eq!(snapshot.note, None);

        let event = create_order_note_added_event("order-1", 2, "No onions please", None);

        let applier = OrderNoteAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.note, Some("No onions please".to_string()));
    }

    #[test]
    fn test_apply_empty_note_clears_snapshot_note() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.note = Some("Existing note".to_string());

        let event =
            create_order_note_added_event("order-1", 2, "", Some("Existing note".to_string()));

        let applier = OrderNoteAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.note, None);
    }

    #[test]
    fn test_apply_note_overwrites_existing() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.note = Some("Old note".to_string());

        let event =
            create_order_note_added_event("order-1", 2, "New note", Some("Old note".to_string()));

        let applier = OrderNoteAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.note, Some("New note".to_string()));
    }

    #[test]
    fn test_updates_sequence_and_timestamp() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.last_sequence = 5;
        snapshot.updated_at = 1000000000;

        let event = create_order_note_added_event("order-1", 10, "Test note", None);
        let expected_timestamp = event.timestamp;

        let applier = OrderNoteAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 10);
        assert_eq!(snapshot.updated_at, expected_timestamp);
    }

    #[test]
    fn test_updates_checksum() {
        let mut snapshot = create_test_snapshot("order-1");
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_order_note_added_event("order-1", 2, "Test note", None);

        let applier = OrderNoteAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }

    #[test]
    fn test_does_not_affect_totals() {
        let mut snapshot = create_test_snapshot("order-1");
        snapshot.subtotal = 100.0;
        snapshot.total = 100.0;
        snapshot.paid_amount = 50.0;
        snapshot.remaining_amount = 50.0;

        let event = create_order_note_added_event("order-1", 2, "Test note", None);

        let applier = OrderNoteAddedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.subtotal, 100.0);
        assert_eq!(snapshot.total, 100.0);
        assert_eq!(snapshot.paid_amount, 50.0);
        assert_eq!(snapshot.remaining_amount, 50.0);
    }

    #[test]
    fn test_wrong_event_type_is_noop() {
        let mut snapshot = create_test_snapshot("order-1");
        let original_note = snapshot.note.clone();
        let original_sequence = snapshot.last_sequence;

        // Create an event with wrong payload type
        let event = OrderEvent::new(
            2,
            "order-1".to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::OrderInfoUpdated,
            EventPayload::OrderInfoUpdated {
                guest_count: Some(5),
                table_name: None,
                is_pre_payment: None,
            },
        );

        let applier = OrderNoteAddedApplier;
        applier.apply(&mut snapshot, &event);

        // Nothing should change
        assert_eq!(snapshot.note, original_note);
        assert_eq!(snapshot.last_sequence, original_sequence);
    }
}
