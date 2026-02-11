//! StampRedeemed event applier
//!
//! Records the stamp redemption event. The actual item comp is handled
//! separately (the frontend/manager will issue a CompItem command for
//! the reward item after stamp redemption succeeds).

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// StampRedeemed applier
pub struct StampRedeemedApplier;

impl EventApplier for StampRedeemedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::StampRedeemed { .. } = &event.payload {
            // StampRedeemed is primarily a record event.
            // The actual comp of the reward item is done via a separate CompItem command.
            // We just update sequence and timestamp.
            snapshot.last_sequence = event.sequence;
            snapshot.updated_at = event.timestamp;

            // Update checksum
            snapshot.update_checksum();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{OrderEventType, OrderSnapshot};

    fn create_stamp_redeemed_event(order_id: &str, seq: u64) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::StampRedeemed,
            EventPayload::StampRedeemed {
                stamp_activity_id: 1,
                stamp_activity_name: "Coffee Card".to_string(),
                reward_item_id: "inst-1".to_string(),
                reward_strategy: "ECONOMIZADOR".to_string(),
            },
        )
    }

    #[test]
    fn test_stamp_redeemed_updates_sequence() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.last_sequence = 5;

        let event = create_stamp_redeemed_event("order-1", 6);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.last_sequence, 6);
    }

    #[test]
    fn test_stamp_redeemed_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_stamp_redeemed_event("order-1", 1);
        let applier = StampRedeemedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }
}
