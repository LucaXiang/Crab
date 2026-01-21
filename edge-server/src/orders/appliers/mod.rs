//! Event applier implementations
//!
//! Each applier implements the `EventApplier` trait and handles
//! one specific event type. Appliers are PURE functions.

use enum_dispatch::enum_dispatch;

use shared::order::{EventPayload, OrderEvent};

mod item_modified;
mod item_removed;
mod items_added;
mod order_completed;
mod order_info_updated;
mod order_voided;
mod payment_added;
mod table_opened;

pub use item_modified::ItemModifiedApplier;
pub use item_removed::ItemRemovedApplier;
pub use items_added::ItemsAddedApplier;
pub use order_completed::OrderCompletedApplier;
pub use order_info_updated::OrderInfoUpdatedApplier;
pub use order_voided::OrderVoidedApplier;
pub use payment_added::PaymentAddedApplier;
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
    PaymentAdded(PaymentAddedApplier),
    OrderCompleted(OrderCompletedApplier),
    OrderInfoUpdated(OrderInfoUpdatedApplier),
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
            EventPayload::OrderCompleted { .. } => {
                EventAction::OrderCompleted(OrderCompletedApplier)
            }
            EventPayload::OrderVoided { .. } => EventAction::OrderVoided(OrderVoidedApplier),
            EventPayload::OrderInfoUpdated { .. } => {
                EventAction::OrderInfoUpdated(OrderInfoUpdatedApplier)
            }
            // Other events will be added here
            _ => todo!("Event applier not yet implemented"),
        }
    }
}
