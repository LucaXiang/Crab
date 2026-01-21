//! Event applier implementations
//!
//! Each applier implements the `EventApplier` trait and handles
//! one specific event type. Appliers are PURE functions.

use enum_dispatch::enum_dispatch;

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

// Applier modules will be added as we implement them
// mod table_opened;
// mod items_added;
// ...

// Re-exports will be added as we implement them
// pub use table_opened::TableOpenedApplier;
// pub use items_added::ItemsAddedApplier;
// ...

/// EventAction enum - dispatches to concrete applier implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(EventApplier)]
pub enum EventAction {
    // Variants will be added as we implement them
    // TableOpened(TableOpenedApplier),
    // ItemsAdded(ItemsAddedApplier),
    // ...
    /// Placeholder variant (remove when first applier is added)
    #[allow(dead_code)]
    Placeholder(PlaceholderApplier),
}

/// Placeholder applier (remove when first applier is added)
pub struct PlaceholderApplier;

impl EventApplier for PlaceholderApplier {
    fn apply(&self, _snapshot: &mut OrderSnapshot, _event: &OrderEvent) {
        unreachable!("PlaceholderApplier should never be called")
    }
}

/// Convert OrderEvent reference to EventAction
///
/// This is the ONLY place with a match on EventPayload.
impl From<&OrderEvent> for EventAction {
    fn from(_event: &OrderEvent) -> Self {
        // Implementation will be added as we implement appliers
        todo!("Implement event conversion")
    }
}
