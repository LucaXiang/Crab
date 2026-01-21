//! Command action implementations
//!
//! Each action implements the `CommandHandler` trait and handles
//! one specific command type.

use enum_dispatch::enum_dispatch;

use shared::order::{OrderCommand, OrderCommandPayload};

mod add_items;
mod complete_order;
mod open_table;

pub use add_items::AddItemsAction;
pub use complete_order::CompleteOrderAction;
pub use open_table::OpenTableAction;

/// CommandAction enum - dispatches to concrete action implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(CommandHandler)]
pub enum CommandAction {
    OpenTable(OpenTableAction),
    AddItems(AddItemsAction),
    CompleteOrder(CompleteOrderAction),
}

/// Convert OrderCommand to CommandAction
///
/// This is the ONLY place with a match on OrderCommandPayload.
impl From<&OrderCommand> for CommandAction {
    fn from(cmd: &OrderCommand) -> Self {
        match &cmd.payload {
            OrderCommandPayload::OpenTable {
                table_id,
                table_name,
                zone_id,
                zone_name,
                guest_count,
                is_retail,
            } => CommandAction::OpenTable(OpenTableAction {
                table_id: table_id.clone(),
                table_name: table_name.clone(),
                zone_id: zone_id.clone(),
                zone_name: zone_name.clone(),
                guest_count: *guest_count,
                is_retail: *is_retail,
            }),
            OrderCommandPayload::AddItems { order_id, items } => {
                CommandAction::AddItems(AddItemsAction {
                    order_id: order_id.clone(),
                    items: items.clone(),
                })
            }
            OrderCommandPayload::CompleteOrder {
                order_id,
                receipt_number,
            } => CommandAction::CompleteOrder(CompleteOrderAction {
                order_id: order_id.clone(),
                receipt_number: receipt_number.clone(),
            }),
            // Other commands will be added here
            _ => todo!("Command not yet implemented"),
        }
    }
}
