//! Command action implementations
//!
//! Each action implements the `CommandHandler` trait and handles
//! one specific command type.

use enum_dispatch::enum_dispatch;

use shared::order::{OrderCommand, OrderCommandPayload};

mod add_items;
mod add_payment;
mod complete_order;
mod modify_item;
mod open_table;
mod remove_item;
mod update_order_info;
mod void_order;

pub use add_items::AddItemsAction;
pub use add_payment::AddPaymentAction;
pub use complete_order::CompleteOrderAction;
pub use modify_item::ModifyItemAction;
pub use open_table::OpenTableAction;
pub use remove_item::RemoveItemAction;
pub use update_order_info::UpdateOrderInfoAction;
pub use void_order::VoidOrderAction;

/// CommandAction enum - dispatches to concrete action implementations
///
/// Uses enum_dispatch for zero-cost static dispatch.
#[enum_dispatch(CommandHandler)]
pub enum CommandAction {
    OpenTable(OpenTableAction),
    AddItems(AddItemsAction),
    ModifyItem(ModifyItemAction),
    RemoveItem(RemoveItemAction),
    AddPayment(AddPaymentAction),
    CompleteOrder(CompleteOrderAction),
    UpdateOrderInfo(UpdateOrderInfoAction),
    VoidOrder(VoidOrderAction),
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
            OrderCommandPayload::ModifyItem {
                order_id,
                instance_id,
                affected_quantity,
                changes,
                authorizer_id,
                authorizer_name,
            } => CommandAction::ModifyItem(ModifyItemAction {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                affected_quantity: *affected_quantity,
                changes: changes.clone(),
                authorizer_id: authorizer_id.clone(),
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::AddPayment { order_id, payment } => {
                CommandAction::AddPayment(AddPaymentAction {
                    order_id: order_id.clone(),
                    payment: payment.clone(),
                })
            }
            OrderCommandPayload::RemoveItem {
                order_id,
                instance_id,
                quantity,
                reason,
                authorizer_id,
                authorizer_name,
            } => CommandAction::RemoveItem(RemoveItemAction {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: *quantity,
                reason: reason.clone(),
                authorizer_id: authorizer_id.clone(),
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::CompleteOrder {
                order_id,
                receipt_number,
            } => CommandAction::CompleteOrder(CompleteOrderAction {
                order_id: order_id.clone(),
                receipt_number: receipt_number.clone(),
            }),
            OrderCommandPayload::VoidOrder { order_id, reason } => {
                CommandAction::VoidOrder(VoidOrderAction {
                    order_id: order_id.clone(),
                    reason: reason.clone(),
                })
            }
            OrderCommandPayload::UpdateOrderInfo {
                order_id,
                receipt_number,
                guest_count,
                table_name,
                is_pre_payment,
            } => CommandAction::UpdateOrderInfo(UpdateOrderInfoAction {
                order_id: order_id.clone(),
                receipt_number: receipt_number.clone(),
                guest_count: *guest_count,
                table_name: table_name.clone(),
                is_pre_payment: *is_pre_payment,
            }),
            // Other commands will be added here
            _ => todo!("Command not yet implemented"),
        }
    }
}
