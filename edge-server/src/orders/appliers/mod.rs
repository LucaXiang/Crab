//! Event applier implementations
//!
//! Each applier implements the `EventApplier` trait and handles
//! one specific event type. Appliers are PURE functions.

use enum_dispatch::enum_dispatch;

use shared::order::{EventPayload, OrderEvent};

mod item_modified;
mod item_removed;
mod item_restored;
mod items_added;
mod order_completed;
mod order_info_updated;
mod order_moved;
mod order_restored;
mod order_voided;
mod payment_added;
mod payment_cancelled;
mod table_opened;

pub use item_modified::ItemModifiedApplier;
pub use item_removed::ItemRemovedApplier;
pub use item_restored::ItemRestoredApplier;
pub use items_added::ItemsAddedApplier;
pub use order_completed::OrderCompletedApplier;
pub use order_info_updated::OrderInfoUpdatedApplier;
pub use order_moved::OrderMovedApplier;
pub use order_restored::OrderRestoredApplier;
pub use order_voided::OrderVoidedApplier;
pub use payment_added::PaymentAddedApplier;
pub use payment_cancelled::PaymentCancelledApplier;
pub use table_opened::TableOpenedApplier;

/// EventAction enum - dispatches to concrete applier implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(EventApplier)]
pub enum EventAction {
    TableOpened(TableOpenedApplier),
    ItemsAdded(ItemsAddedApplier),
    ItemModified(ItemModifiedApplier),
    ItemRemoved(ItemRemovedApplier),
    ItemRestored(ItemRestoredApplier),
    PaymentAdded(PaymentAddedApplier),
    PaymentCancelled(PaymentCancelledApplier),
    OrderCompleted(OrderCompletedApplier),
    OrderInfoUpdated(OrderInfoUpdatedApplier),
    OrderMoved(OrderMovedApplier),
    OrderRestored(OrderRestoredApplier),
    OrderVoided(OrderVoidedApplier),
}

/// Convert OrderEvent reference to EventAction
///
/// This is the ONLY place with a match on EventPayload.
impl From<&OrderEvent> for EventAction {
    fn from(event: &OrderEvent) -> Self {
        match &event.payload {
            EventPayload::TableOpened { .. } => EventAction::TableOpened(TableOpenedApplier),
            EventPayload::ItemsAdded { .. } => EventAction::ItemsAdded(ItemsAddedApplier),
            EventPayload::ItemModified { .. } => EventAction::ItemModified(ItemModifiedApplier),
            EventPayload::ItemRemoved { .. } => EventAction::ItemRemoved(ItemRemovedApplier),
            EventPayload::PaymentAdded { .. } => EventAction::PaymentAdded(PaymentAddedApplier),
            EventPayload::PaymentCancelled { .. } => {
                EventAction::PaymentCancelled(PaymentCancelledApplier)
            }
            EventPayload::OrderCompleted { .. } => {
                EventAction::OrderCompleted(OrderCompletedApplier)
            }
            EventPayload::OrderVoided { .. } => EventAction::OrderVoided(OrderVoidedApplier),
            EventPayload::OrderRestored { .. } => EventAction::OrderRestored(OrderRestoredApplier),
            EventPayload::OrderInfoUpdated { .. } => {
                EventAction::OrderInfoUpdated(OrderInfoUpdatedApplier)
            }
            EventPayload::OrderMoved { .. } => EventAction::OrderMoved(OrderMovedApplier),
            EventPayload::ItemRestored { .. } => EventAction::ItemRestored(ItemRestoredApplier),
            // Other events will be added here
            _ => todo!("Event applier not yet implemented"),
        }
    }
}
