//! Command action implementations
//!
//! Each action implements the `CommandHandler` trait and handles
//! one specific command type.

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{OrderCommand, OrderCommandPayload, OrderEvent};

mod add_items;
mod add_order_note;
mod add_payment;
mod apply_order_adjustment;
mod cancel_payment;
mod cancel_stamp_redemption;
mod comp_item;
mod complete_order;
mod link_member;
mod merge_orders;
mod modify_item;
mod move_order;
pub mod open_table;
mod redeem_stamp;
mod remove_item;
mod split_order;
mod toggle_rule_skip;
mod uncomp_item;
mod unlink_member;
mod update_order_info;
mod void_order;

pub use add_items::AddItemsAction;
pub use add_order_note::AddOrderNoteAction;
pub use add_payment::AddPaymentAction;
pub use apply_order_adjustment::{ApplyOrderDiscountAction, ApplyOrderSurchargeAction};
pub use cancel_payment::CancelPaymentAction;
pub use cancel_stamp_redemption::CancelStampRedemptionAction;
pub use comp_item::CompItemAction;
pub use complete_order::CompleteOrderAction;
pub use link_member::LinkMemberAction;
pub use merge_orders::MergeOrdersAction;
pub use modify_item::ModifyItemAction;
pub use move_order::MoveOrderAction;
pub use open_table::OpenTableAction;
pub use redeem_stamp::{RedeemStampAction, RewardProductInfo};

pub use remove_item::RemoveItemAction;
pub use split_order::{
    PayAaSplitAction, SplitByAmountAction, SplitByItemsAction, StartAaSplitAction,
};
pub use toggle_rule_skip::ToggleRuleSkipAction;
pub use uncomp_item::UncompItemAction;
pub use unlink_member::UnlinkMemberAction;
pub use update_order_info::UpdateOrderInfoAction;
pub use void_order::VoidOrderAction;

/// CommandAction enum - dispatches to concrete action implementations
pub enum CommandAction {
    OpenTable(OpenTableAction),
    AddItems(AddItemsAction),
    ModifyItem(ModifyItemAction),
    RemoveItem(RemoveItemAction),
    CompItem(CompItemAction),
    UncompItem(UncompItemAction),
    AddPayment(AddPaymentAction),
    CancelPayment(CancelPaymentAction),
    CompleteOrder(CompleteOrderAction),
    UpdateOrderInfo(UpdateOrderInfoAction),
    VoidOrder(VoidOrderAction),
    MoveOrder(MoveOrderAction),
    MergeOrders(MergeOrdersAction),
    SplitByItems(SplitByItemsAction),
    SplitByAmount(SplitByAmountAction),
    StartAaSplit(StartAaSplitAction),
    PayAaSplit(PayAaSplitAction),
    ToggleRuleSkip(ToggleRuleSkipAction),
    ApplyOrderDiscount(ApplyOrderDiscountAction),
    ApplyOrderSurcharge(ApplyOrderSurchargeAction),
    AddOrderNote(AddOrderNoteAction),
    LinkMember(LinkMemberAction),
    UnlinkMember(UnlinkMemberAction),
    RedeemStamp(RedeemStampAction),
    CancelStampRedemption(CancelStampRedemptionAction),
}

/// Manual implementation of CommandHandler for CommandAction
impl CommandHandler for CommandAction {
    fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        match self {
            CommandAction::OpenTable(action) => action.execute(ctx, metadata),
            CommandAction::AddItems(action) => action.execute(ctx, metadata),
            CommandAction::ModifyItem(action) => action.execute(ctx, metadata),
            CommandAction::RemoveItem(action) => action.execute(ctx, metadata),
            CommandAction::CompItem(action) => action.execute(ctx, metadata),
            CommandAction::UncompItem(action) => action.execute(ctx, metadata),
            CommandAction::AddPayment(action) => action.execute(ctx, metadata),
            CommandAction::CancelPayment(action) => action.execute(ctx, metadata),
            CommandAction::CompleteOrder(action) => action.execute(ctx, metadata),
            CommandAction::UpdateOrderInfo(action) => action.execute(ctx, metadata),
            CommandAction::VoidOrder(action) => action.execute(ctx, metadata),
            CommandAction::MoveOrder(action) => action.execute(ctx, metadata),
            CommandAction::MergeOrders(action) => action.execute(ctx, metadata),
            CommandAction::SplitByItems(action) => action.execute(ctx, metadata),
            CommandAction::SplitByAmount(action) => action.execute(ctx, metadata),
            CommandAction::StartAaSplit(action) => action.execute(ctx, metadata),
            CommandAction::PayAaSplit(action) => action.execute(ctx, metadata),
            CommandAction::ToggleRuleSkip(action) => action.execute(ctx, metadata),
            CommandAction::ApplyOrderDiscount(action) => action.execute(ctx, metadata),
            CommandAction::ApplyOrderSurcharge(action) => action.execute(ctx, metadata),
            CommandAction::AddOrderNote(action) => action.execute(ctx, metadata),
            CommandAction::LinkMember(action) => action.execute(ctx, metadata),
            CommandAction::UnlinkMember(action) => action.execute(ctx, metadata),
            CommandAction::RedeemStamp(action) => action.execute(ctx, metadata),
            CommandAction::CancelStampRedemption(action) => action.execute(ctx, metadata),
        }
    }
}

/// Convert OrderCommand to CommandAction
///
/// 通用命令→Action 转换。需要 prefetch 数据注入的命令在 OrdersManager::process_command() 中直接构建。
impl From<&OrderCommand> for CommandAction {
    fn from(cmd: &OrderCommand) -> Self {
        match &cmd.payload {
            OrderCommandPayload::OpenTable { .. } => {
                // OpenTable is handled specially in OrdersManager to generate receipt_number
                // This path should never be reached
                unreachable!(
                    "OpenTable should be handled by OrdersManager, not From<&OrderCommand>"
                )
            }
            OrderCommandPayload::AddItems { .. } => {
                // AddItems is handled specially in OrdersManager to inject rules and metadata
                unreachable!("AddItems should be handled by OrdersManager, not From<&OrderCommand>")
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
                authorizer_id: *authorizer_id,
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
                authorizer_id: *authorizer_id,
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
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::CompleteOrder {
                order_id,
                service_type,
            } => CommandAction::CompleteOrder(CompleteOrderAction {
                order_id: order_id.clone(),
                service_type: *service_type,
            }),
            OrderCommandPayload::VoidOrder {
                order_id,
                void_type,
                loss_reason,
                loss_amount,
                note,
                authorizer_id,
                authorizer_name,
            } => CommandAction::VoidOrder(VoidOrderAction {
                order_id: order_id.clone(),
                void_type: void_type.clone(),
                loss_reason: loss_reason.clone(),
                loss_amount: *loss_amount,
                note: note.clone(),
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::UpdateOrderInfo {
                order_id,
                guest_count,
                table_name,
                is_pre_payment,
            } => CommandAction::UpdateOrderInfo(UpdateOrderInfoAction {
                order_id: order_id.clone(),
                guest_count: *guest_count,
                table_name: table_name.clone(),
                is_pre_payment: *is_pre_payment,
            }),
            OrderCommandPayload::MoveOrder {
                order_id,
                target_table_id,
                target_table_name,
                target_zone_id,
                target_zone_name,
                authorizer_id,
                authorizer_name,
            } => CommandAction::MoveOrder(MoveOrderAction {
                order_id: order_id.clone(),
                target_table_id: *target_table_id,
                target_table_name: target_table_name.clone(),
                target_zone_id: *target_zone_id,
                target_zone_name: target_zone_name.clone(),
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::MergeOrders {
                source_order_id,
                target_order_id,
                authorizer_id,
                authorizer_name,
            } => CommandAction::MergeOrders(MergeOrdersAction {
                source_order_id: source_order_id.clone(),
                target_order_id: target_order_id.clone(),
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::SplitByItems {
                order_id,
                payment_method,
                items,
                tendered,
            } => CommandAction::SplitByItems(SplitByItemsAction {
                order_id: order_id.clone(),
                payment_method: payment_method.clone(),
                items: items.clone(),
                tendered: *tendered,
            }),
            OrderCommandPayload::SplitByAmount {
                order_id,
                split_amount,
                payment_method,
                tendered,
            } => CommandAction::SplitByAmount(SplitByAmountAction {
                order_id: order_id.clone(),
                split_amount: *split_amount,
                payment_method: payment_method.clone(),
                tendered: *tendered,
            }),
            OrderCommandPayload::StartAaSplit {
                order_id,
                total_shares,
                shares,
                payment_method,
                tendered,
            } => CommandAction::StartAaSplit(StartAaSplitAction {
                order_id: order_id.clone(),
                total_shares: *total_shares,
                shares: *shares,
                payment_method: payment_method.clone(),
                tendered: *tendered,
            }),
            OrderCommandPayload::PayAaSplit {
                order_id,
                shares,
                payment_method,
                tendered,
            } => CommandAction::PayAaSplit(PayAaSplitAction {
                order_id: order_id.clone(),
                shares: *shares,
                payment_method: payment_method.clone(),
                tendered: *tendered,
            }),
            OrderCommandPayload::CompItem {
                order_id,
                instance_id,
                quantity,
                reason,
                authorizer_id,
                authorizer_name,
            } => CommandAction::CompItem(CompItemAction {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                quantity: *quantity,
                reason: reason.clone(),
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::UncompItem {
                order_id,
                instance_id,
                authorizer_id,
                authorizer_name,
            } => CommandAction::UncompItem(UncompItemAction {
                order_id: order_id.clone(),
                instance_id: instance_id.clone(),
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::ToggleRuleSkip {
                order_id,
                rule_id,
                skipped,
            } => CommandAction::ToggleRuleSkip(ToggleRuleSkipAction {
                order_id: order_id.clone(),
                rule_id: *rule_id,
                skipped: *skipped,
            }),
            OrderCommandPayload::ApplyOrderDiscount {
                order_id,
                discount_percent,
                discount_fixed,
                authorizer_id,
                authorizer_name,
            } => CommandAction::ApplyOrderDiscount(ApplyOrderDiscountAction {
                order_id: order_id.clone(),
                discount_percent: *discount_percent,
                discount_fixed: *discount_fixed,
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::ApplyOrderSurcharge {
                order_id,
                surcharge_percent,
                surcharge_amount,
                authorizer_id,
                authorizer_name,
            } => CommandAction::ApplyOrderSurcharge(ApplyOrderSurchargeAction {
                order_id: order_id.clone(),
                surcharge_percent: *surcharge_percent,
                surcharge_amount: *surcharge_amount,
                authorizer_id: *authorizer_id,
                authorizer_name: authorizer_name.clone(),
            }),
            OrderCommandPayload::AddOrderNote { order_id, note } => {
                CommandAction::AddOrderNote(AddOrderNoteAction {
                    order_id: order_id.clone(),
                    note: note.clone(),
                })
            }
            OrderCommandPayload::LinkMember { .. } => {
                // LinkMember requires data injection (member info, MG rules)
                // Handled specially in OrdersManager, not via From<&OrderCommand>
                unreachable!(
                    "LinkMember should be handled by OrdersManager, not From<&OrderCommand>"
                )
            }
            OrderCommandPayload::UnlinkMember { order_id } => {
                CommandAction::UnlinkMember(UnlinkMemberAction {
                    order_id: order_id.clone(),
                })
            }
            OrderCommandPayload::RedeemStamp { .. } => {
                // RedeemStamp requires data injection (activity, reward targets, product info)
                // Handled specially in OrdersManager, not via From<&OrderCommand>
                unreachable!(
                    "RedeemStamp should be handled by OrdersManager, not From<&OrderCommand>"
                )
            }
            OrderCommandPayload::CancelStampRedemption {
                order_id,
                stamp_activity_id,
            } => CommandAction::CancelStampRedemption(CancelStampRedemptionAction {
                order_id: order_id.clone(),
                stamp_activity_id: *stamp_activity_id,
            }),
        }
    }
}
