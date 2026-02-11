//! MemberLinked event applier
//!
//! Applies the MemberLinked event to set member info on the snapshot.
//! MG discount recalculation happens via recalculate_totals in the future
//! when MG rules are injected into items.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

/// MemberLinked applier
pub struct MemberLinkedApplier;

impl EventApplier for MemberLinkedApplier {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        if let EventPayload::MemberLinked {
            member_id,
            member_name,
            marketing_group_id,
            marketing_group_name,
        } = &event.payload
        {
            snapshot.member_id = Some(*member_id);
            snapshot.member_name = Some(member_name.clone());
            snapshot.marketing_group_id = Some(*marketing_group_id);
            snapshot.marketing_group_name = Some(marketing_group_name.clone());

            // Update sequence and timestamp
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

    fn create_member_linked_event(
        order_id: &str,
        seq: u64,
        member_id: i64,
        member_name: &str,
        mg_id: i64,
        mg_name: &str,
    ) -> OrderEvent {
        OrderEvent::new(
            seq,
            order_id.to_string(),
            1,
            "Test User".to_string(),
            "cmd-1".to_string(),
            Some(1234567890),
            OrderEventType::MemberLinked,
            EventPayload::MemberLinked {
                member_id,
                member_name: member_name.to_string(),
                marketing_group_id: mg_id,
                marketing_group_name: mg_name.to_string(),
            },
        )
    }

    #[test]
    fn test_member_linked_sets_fields() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        assert!(snapshot.member_id.is_none());
        assert!(snapshot.marketing_group_id.is_none());

        let event = create_member_linked_event("order-1", 2, 42, "Alice", 1, "VIP");
        let applier = MemberLinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.member_id, Some(42));
        assert_eq!(snapshot.member_name, Some("Alice".to_string()));
        assert_eq!(snapshot.marketing_group_id, Some(1));
        assert_eq!(snapshot.marketing_group_name, Some("VIP".to_string()));
        assert_eq!(snapshot.last_sequence, 2);
    }

    #[test]
    fn test_member_linked_replaces_existing() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.member_id = Some(10);
        snapshot.member_name = Some("Bob".to_string());
        snapshot.marketing_group_id = Some(5);
        snapshot.marketing_group_name = Some("Regular".to_string());

        let event = create_member_linked_event("order-1", 3, 42, "Alice", 1, "VIP");
        let applier = MemberLinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_eq!(snapshot.member_id, Some(42));
        assert_eq!(snapshot.member_name, Some("Alice".to_string()));
        assert_eq!(snapshot.marketing_group_id, Some(1));
        assert_eq!(snapshot.marketing_group_name, Some("VIP".to_string()));
    }

    #[test]
    fn test_member_linked_updates_checksum() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        let initial_checksum = snapshot.state_checksum.clone();

        let event = create_member_linked_event("order-1", 1, 42, "Alice", 1, "VIP");
        let applier = MemberLinkedApplier;
        applier.apply(&mut snapshot, &event);

        assert_ne!(snapshot.state_checksum, initial_checksum);
        assert!(snapshot.verify_checksum());
    }
}
