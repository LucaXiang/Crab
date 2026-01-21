//! Event applier implementations
//!
//! Each applier implements the `EventApplier` trait and handles
//! one specific event type. Appliers are PURE functions.

use enum_dispatch::enum_dispatch;

use shared::order::{EventPayload, OrderEvent};

mod table_opened;

pub use table_opened::TableOpenedApplier;

/// EventAction enum - dispatches to concrete applier implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(EventApplier)]
pub enum EventAction {
    TableOpened(TableOpenedApplier),
}

/// Convert OrderEvent reference to EventAction
///
/// This is the ONLY place with a match on EventPayload.
impl From<&OrderEvent> for EventAction {
    fn from(event: &OrderEvent) -> Self {
        match &event.payload {
            EventPayload::TableOpened { .. } => EventAction::TableOpened(TableOpenedApplier),
            // Other events will be added here
            _ => todo!("Event applier not yet implemented"),
        }
    }
}
