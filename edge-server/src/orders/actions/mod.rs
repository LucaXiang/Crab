//! Command action implementations
//!
//! Each action implements the `CommandHandler` trait and handles
//! one specific command type.

use async_trait::async_trait;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{OrderCommand, OrderCommandPayload, OrderEvent};

mod add_items;
mod add_payment;
mod cancel_payment;
mod complete_order;
mod merge_orders;
mod modify_item;
mod move_order;
pub mod open_table;
mod remove_item;
mod restore_item;
mod restore_order;
mod split_order;
mod toggle_rule_skip;
mod update_order_info;
mod void_order;

pub use add_items::AddItemsAction;
pub use add_payment::AddPaymentAction;
pub use cancel_payment::CancelPaymentAction;
pub use complete_order::CompleteOrderAction;
pub use merge_orders::MergeOrdersAction;
pub use modify_item::ModifyItemAction;
pub use move_order::MoveOrderAction;
pub use open_table::OpenTableAction;
pub use remove_item::RemoveItemAction;
pub use restore_item::RestoreItemAction;
pub use restore_order::RestoreOrderAction;
pub use split_order::SplitOrderAction;
pub use toggle_rule_skip::ToggleRuleSkipAction;
pub use update_order_info::UpdateOrderInfoAction;
pub use void_order::VoidOrderAction;

/// CommandAction enum - dispatches to concrete action implementations
pub enum CommandAction {
    OpenTable(OpenTableAction),
    AddItems(AddItemsAction),
    ModifyItem(ModifyItemAction),
    RemoveItem(RemoveItemAction),
    RestoreItem(RestoreItemAction),
    AddPayment(AddPaymentAction),
    CancelPayment(CancelPaymentAction),
    CompleteOrder(CompleteOrderAction),
    UpdateOrderInfo(UpdateOrderInfoAction),
    VoidOrder(VoidOrderAction),
    RestoreOrder(RestoreOrderAction),
    MoveOrder(MoveOrderAction),
    MergeOrders(MergeOrdersAction),
    SplitOrder(SplitOrderAction),
    ToggleRuleSkip(ToggleRuleSkipAction),
}

/// Manual implementation of CommandHandler for CommandAction
#[async_trait]
impl CommandHandler for CommandAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        match self {
            CommandAction::OpenTable(action) => action.execute(ctx, metadata).await,
            CommandAction::AddItems(action) => action.execute(ctx, metadata).await,
            CommandAction::ModifyItem(action) => action.execute(ctx, metadata).await,
            CommandAction::RemoveItem(action) => action.execute(ctx, metadata).await,
            CommandAction::RestoreItem(action) => action.execute(ctx, metadata).await,
            CommandAction::AddPayment(action) => action.execute(ctx, metadata).await,
            CommandAction::CancelPayment(action) => action.execute(ctx, metadata).await,
            CommandAction::CompleteOrder(action) => action.execute(ctx, metadata).await,
            CommandAction::UpdateOrderInfo(action) => action.execute(ctx, metadata).await,
            CommandAction::VoidOrder(action) => action.execute(ctx, metadata).await,
            CommandAction::RestoreOrder(action) => action.execute(ctx, metadata).await,
            CommandAction::MoveOrder(action) => action.execute(ctx, metadata).await,
            CommandAction::MergeOrders(action) => action.execute(ctx, metadata).await,
            CommandAction::SplitOrder(action) => action.execute(ctx, metadata).await,
            CommandAction::ToggleRuleSkip(action) => action.execute(ctx, metadata).await,
        }
    }
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
                    rules: vec![], // Rules will be injected by OrdersManager
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
            OrderCommandPayload::CancelPayment {
                order_id,
                payment_id,
                reason,
                authorizer_id,
                authorizer_name,
            } => CommandAction::CancelPayment(CancelPaymentAction {
                order_id: order_id.clone(),
                payment_id: payment_id.clone(),
                reason: reason.clone(),
                authorizer_id: authorizer_id.clone(),
                authorizer_name: authorizer_name.clone(),
            }),
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
            OrderCommandPayload::RestoreOrder { order_id } => {
                CommandAction::RestoreOrder(RestoreOrderAction {
                    order_id: order_id.clone(),
                })
            }
            OrderCommandPayload::RestoreItem {
                order_id,
                instance_id,
            } => CommandAction::RestoreItem(RestoreItemAction {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
            }),
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
            OrderCommandPayload::MoveOrder {
                order_id,
                target_table_id,
                target_table_name,
                target_zone_name,
            } => CommandAction::MoveOrder(MoveOrderAction {
                order_id: order_id.clone(),
                target_table_id: target_table_id.clone(),
                target_table_name: target_table_name.clone(),
                target_zone_id: None, // Not in OrderCommandPayload
                target_zone_name: target_zone_name.clone(),
            }),
            OrderCommandPayload::MergeOrders {
                source_order_id,
                target_order_id,
            } => CommandAction::MergeOrders(MergeOrdersAction {
                source_order_id: source_order_id.clone(),
                target_order_id: target_order_id.clone(),
            }),
            OrderCommandPayload::SplitOrder {
                order_id,
                split_amount,
                payment_method,
                items,
            } => CommandAction::SplitOrder(SplitOrderAction {
                order_id: order_id.clone(),
                split_amount: *split_amount,
                payment_method: payment_method.clone(),
                items: items.clone(),
            }),
            OrderCommandPayload::ToggleRuleSkip {
                order_id,
                rule_id,
                skipped,
            } => CommandAction::ToggleRuleSkip(ToggleRuleSkipAction {
                order_id: order_id.clone(),
                rule_id: rule_id.clone(),
                skipped: *skipped,
            }),
        }
    }
}
