//! AddOrderNote command handler
//!
//! Adds or clears an order-level note. Replaces the previous note (not append).
//! Empty string clears the note. No authorization required.

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use crate::utils::validation::{MAX_NOTE_LEN, validate_order_text};
use shared::order::types::CommandErrorCode;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// AddOrderNote action
#[derive(Debug, Clone)]
pub struct AddOrderNoteAction {
    pub order_id: i64,
    pub note: String,
}

impl CommandHandler for AddOrderNoteAction {
    fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Validate text length (empty string is allowed — it clears the note)
        if !self.note.is_empty() {
            validate_order_text(&self.note, "note", MAX_NOTE_LEN)?;
        }

        // 2. Load existing snapshot
        let snapshot = ctx.load_snapshot(self.order_id)?;

        // 3. Validate order status - must be Active
        match snapshot.status {
            OrderStatus::Active => {}
            OrderStatus::Completed => {
                return Err(OrderError::OrderAlreadyCompleted(self.order_id));
            }
            OrderStatus::Void => {
                return Err(OrderError::OrderAlreadyVoided(self.order_id));
            }
            _ => {
                return Err(OrderError::InvalidOperation(
                    CommandErrorCode::OrderNotActive,
                    format!(
                        "Cannot add note to order with status: {:?}",
                        snapshot.status
                    ),
                ));
            }
        }

        // 4. Capture previous note for audit trail
        let previous_note = snapshot.note.clone();

        // 5. Allocate sequence number
        let seq = ctx.next_sequence();

        // 6. Create event
        let event = OrderEvent::new(
            seq,
            self.order_id,
            metadata.operator_id,
            metadata.operator_name.clone(),
            metadata.command_id,
            Some(metadata.timestamp),
            OrderEventType::OrderNoteAdded,
            EventPayload::OrderNoteAdded {
                note: self.note.clone(),
                previous_note,
            },
        );

        Ok(vec![event])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::storage::OrderStorage;
    use crate::orders::traits::CommandContext;
    use shared::order::OrderSnapshot;

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: 1,
            operator_id: 1,
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    fn create_active_order(order_id: i64) -> OrderSnapshot {
        let mut snapshot = OrderSnapshot::new(order_id);
        snapshot.status = OrderStatus::Active;
        snapshot
    }

    #[test]
    fn test_add_note_to_active_order_succeeds() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order(1001);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddOrderNoteAction {
            order_id: 1001,
            note: "No onions please".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.order_id, 1001);
        assert_eq!(event.event_type, OrderEventType::OrderNoteAdded);

        if let EventPayload::OrderNoteAdded {
            note,
            previous_note,
        } = &event.payload
        {
            assert_eq!(note, "No onions please");
            assert_eq!(*previous_note, None);
        } else {
            panic!("Expected OrderNoteAdded payload");
        }
    }

    #[test]
    fn test_add_note_to_completed_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(1001);
        snapshot.status = OrderStatus::Completed;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddOrderNoteAction {
            order_id: 1001,
            note: "Test note".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::OrderAlreadyCompleted(_))));
    }

    #[test]
    fn test_add_note_to_voided_order_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = OrderSnapshot::new(1001);
        snapshot.status = OrderStatus::Void;
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddOrderNoteAction {
            order_id: 1001,
            note: "Test note".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::OrderAlreadyVoided(_))));
    }

    #[test]
    fn test_add_note_captures_previous_note() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order(1001);
        snapshot.note = Some("Old note".to_string());
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddOrderNoteAction {
            order_id: 1001,
            note: "New note".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        if let EventPayload::OrderNoteAdded {
            note,
            previous_note,
        } = &events[0].payload
        {
            assert_eq!(note, "New note");
            assert_eq!(*previous_note, Some("Old note".to_string()));
        } else {
            panic!("Expected OrderNoteAdded payload");
        }
    }

    #[test]
    fn test_clear_note_with_empty_string() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let mut snapshot = create_active_order(1001);
        snapshot.note = Some("Existing note".to_string());
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddOrderNoteAction {
            order_id: 1001,
            note: "".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::OrderNoteAdded {
            note,
            previous_note,
        } = &events[0].payload
        {
            assert_eq!(note, "");
            assert_eq!(*previous_note, Some("Existing note".to_string()));
        } else {
            panic!("Expected OrderNoteAdded payload");
        }
    }

    #[test]
    fn test_add_note_generates_correct_event_payload() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let snapshot = create_active_order(1001);
        storage.store_snapshot(&txn, &snapshot).unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddOrderNoteAction {
            order_id: 1001,
            note: "Special request".to_string(),
        };

        let metadata = CommandMetadata {
            command_id: 123,
            operator_id: 456,
            operator_name: "John Doe".to_string(),
            timestamp: 9999999999,
        };

        let events = action.execute(&mut ctx, &metadata).unwrap();

        let event = &events[0];
        assert_eq!(event.command_id, 123);
        assert_eq!(event.operator_id, 456);
        assert_eq!(event.operator_name, "John Doe");
        assert_eq!(event.sequence, current_seq + 1);
        assert_eq!(event.event_type, OrderEventType::OrderNoteAdded);
    }

    #[test]
    fn test_add_note_order_not_found_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = AddOrderNoteAction {
            order_id: 9999,
            note: "Test".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata);
        assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
    }
}
