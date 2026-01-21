//! Command action implementations
//!
//! Each action implements the `CommandHandler` trait and handles
//! one specific command type.

use enum_dispatch::enum_dispatch;

use crate::orders::traits::CommandHandler;
use shared::order::{OrderCommand, OrderCommandPayload};

// Action modules will be added as we implement them
// mod open_table;
// mod add_items;
// ...

// Re-exports will be added as we implement them
// pub use open_table::OpenTableAction;
// pub use add_items::AddItemsAction;
// ...

/// CommandAction enum - dispatches to concrete action implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(CommandHandler)]
pub enum CommandAction {
    // Variants will be added as we implement them
    // OpenTable(OpenTableAction),
    // AddItems(AddItemsAction),
    // ...
    /// Placeholder variant (remove when first action is added)
    #[allow(dead_code)]
    Placeholder(PlaceholderAction),
}

/// Placeholder action (remove when first action is added)
pub struct PlaceholderAction;

#[async_trait::async_trait]
impl CommandHandler for PlaceholderAction {
    async fn execute(
        &self,
        _ctx: &mut crate::orders::traits::CommandContext<'_>,
        _metadata: &crate::orders::traits::CommandMetadata,
    ) -> Result<Vec<shared::order::OrderEvent>, crate::orders::traits::OrderError> {
        unreachable!("PlaceholderAction should never be executed")
    }
}

/// Convert OrderCommand to CommandAction
///
/// This is the ONLY place with a match on OrderCommandPayload.
impl From<&OrderCommand> for CommandAction {
    fn from(_cmd: &OrderCommand) -> Self {
        // Implementation will be added as we implement actions
        todo!("Implement command conversion")
    }
}
