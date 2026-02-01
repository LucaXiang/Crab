//! Event applier implementations
//!
//! Each applier implements the `EventApplier` trait and handles
//! one specific event type. Appliers are PURE functions.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

mod item_comped;
mod item_modified;
mod item_uncomped;
mod item_removed;
mod item_restored;
mod items_added;
mod order_adjustment_applied;
mod order_completed;
mod order_info_updated;
mod order_moved;
mod order_moved_out;
mod order_note_added;
mod order_split;
mod order_voided;
mod orders_merged;
mod payment_added;
mod payment_cancelled;
mod rule_skip_toggled;
mod table_opened;
mod table_reassigned;

pub use item_comped::ItemCompedApplier;
pub use item_modified::ItemModifiedApplier;
pub use item_uncomped::ItemUncompedApplier;
pub use item_removed::ItemRemovedApplier;
pub use item_restored::ItemRestoredApplier;
pub use items_added::ItemsAddedApplier;
pub use order_adjustment_applied::{OrderDiscountAppliedApplier, OrderSurchargeAppliedApplier};
pub use order_completed::OrderCompletedApplier;
pub use order_info_updated::OrderInfoUpdatedApplier;
pub use order_moved::OrderMovedApplier;
pub use order_note_added::OrderNoteAddedApplier;
pub use order_moved_out::OrderMovedOutApplier;
pub use order_split::{
    AaSplitCancelledApplier, AaSplitPaidApplier, AaSplitStartedApplier, AmountSplitApplier,
    ItemSplitApplier,
};
pub use order_voided::OrderVoidedApplier;
pub use orders_merged::{OrderMergedApplier, OrderMergedOutApplier};
pub use payment_added::PaymentAddedApplier;
pub use payment_cancelled::PaymentCancelledApplier;
pub use rule_skip_toggled::RuleSkipToggledApplier;
pub use table_opened::TableOpenedApplier;
pub use table_reassigned::TableReassignedApplier;

/// EventAction enum - dispatches to concrete applier implementations
pub enum EventAction {
    TableOpened(TableOpenedApplier),
    ItemsAdded(ItemsAddedApplier),
    ItemModified(ItemModifiedApplier),
    ItemRemoved(ItemRemovedApplier),
    ItemRestored(ItemRestoredApplier),
    ItemComped(ItemCompedApplier),
    ItemUncomped(ItemUncompedApplier),
    PaymentAdded(PaymentAddedApplier),
    PaymentCancelled(PaymentCancelledApplier),
    OrderCompleted(OrderCompletedApplier),
    OrderInfoUpdated(OrderInfoUpdatedApplier),
    OrderMoved(OrderMovedApplier),
    OrderMovedOut(OrderMovedOutApplier),
    OrderVoided(OrderVoidedApplier),
    OrderMerged(OrderMergedApplier),
    OrderMergedOut(OrderMergedOutApplier),
    ItemSplit(ItemSplitApplier),
    AmountSplit(AmountSplitApplier),
    AaSplitStarted(AaSplitStartedApplier),
    AaSplitPaid(AaSplitPaidApplier),
    AaSplitCancelled(AaSplitCancelledApplier),
    RuleSkipToggled(RuleSkipToggledApplier),
    TableReassigned(TableReassignedApplier),
    OrderDiscountApplied(OrderDiscountAppliedApplier),
    OrderSurchargeApplied(OrderSurchargeAppliedApplier),
    OrderNoteAdded(OrderNoteAddedApplier),
}

/// Manual implementation of EventApplier for EventAction
impl EventApplier for EventAction {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        match self {
            EventAction::TableOpened(applier) => applier.apply(snapshot, event),
            EventAction::ItemsAdded(applier) => applier.apply(snapshot, event),
            EventAction::ItemModified(applier) => applier.apply(snapshot, event),
            EventAction::ItemRemoved(applier) => applier.apply(snapshot, event),
            EventAction::ItemRestored(applier) => applier.apply(snapshot, event),
            EventAction::ItemComped(applier) => applier.apply(snapshot, event),
            EventAction::ItemUncomped(applier) => applier.apply(snapshot, event),
            EventAction::PaymentAdded(applier) => applier.apply(snapshot, event),
            EventAction::PaymentCancelled(applier) => applier.apply(snapshot, event),
            EventAction::OrderCompleted(applier) => applier.apply(snapshot, event),
            EventAction::OrderInfoUpdated(applier) => applier.apply(snapshot, event),
            EventAction::OrderMoved(applier) => applier.apply(snapshot, event),
            EventAction::OrderMovedOut(applier) => applier.apply(snapshot, event),
            EventAction::OrderVoided(applier) => applier.apply(snapshot, event),
            EventAction::OrderMerged(applier) => applier.apply(snapshot, event),
            EventAction::OrderMergedOut(applier) => applier.apply(snapshot, event),
            EventAction::ItemSplit(applier) => applier.apply(snapshot, event),
            EventAction::AmountSplit(applier) => applier.apply(snapshot, event),
            EventAction::AaSplitStarted(applier) => applier.apply(snapshot, event),
            EventAction::AaSplitPaid(applier) => applier.apply(snapshot, event),
            EventAction::AaSplitCancelled(applier) => applier.apply(snapshot, event),
            EventAction::RuleSkipToggled(applier) => applier.apply(snapshot, event),
            EventAction::TableReassigned(applier) => applier.apply(snapshot, event),
            EventAction::OrderDiscountApplied(applier) => applier.apply(snapshot, event),
            EventAction::OrderSurchargeApplied(applier) => applier.apply(snapshot, event),
            EventAction::OrderNoteAdded(applier) => applier.apply(snapshot, event),
        }
    }
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
            EventPayload::OrderInfoUpdated { .. } => {
                EventAction::OrderInfoUpdated(OrderInfoUpdatedApplier)
            }
            EventPayload::OrderMoved { .. } => EventAction::OrderMoved(OrderMovedApplier),
            EventPayload::ItemRestored { .. } => EventAction::ItemRestored(ItemRestoredApplier),
            EventPayload::ItemComped { .. } => EventAction::ItemComped(ItemCompedApplier),
            EventPayload::ItemUncomped { .. } => EventAction::ItemUncomped(ItemUncompedApplier),
            EventPayload::OrderMerged { .. } => EventAction::OrderMerged(OrderMergedApplier),
            EventPayload::OrderMergedOut { .. } => {
                EventAction::OrderMergedOut(OrderMergedOutApplier)
            }
            EventPayload::OrderMovedOut { .. } => {
                EventAction::OrderMovedOut(OrderMovedOutApplier)
            }
            EventPayload::ItemSplit { .. } => EventAction::ItemSplit(ItemSplitApplier),
            EventPayload::AmountSplit { .. } => EventAction::AmountSplit(AmountSplitApplier),
            EventPayload::AaSplitStarted { .. } => {
                EventAction::AaSplitStarted(AaSplitStartedApplier)
            }
            EventPayload::AaSplitPaid { .. } => EventAction::AaSplitPaid(AaSplitPaidApplier),
            EventPayload::AaSplitCancelled { .. } => {
                EventAction::AaSplitCancelled(AaSplitCancelledApplier)
            }
            EventPayload::RuleSkipToggled { .. } => {
                EventAction::RuleSkipToggled(RuleSkipToggledApplier)
            }
            EventPayload::TableReassigned { .. } => {
                EventAction::TableReassigned(TableReassignedApplier)
            }
            EventPayload::OrderDiscountApplied { .. } => {
                EventAction::OrderDiscountApplied(OrderDiscountAppliedApplier)
            }
            EventPayload::OrderSurchargeApplied { .. } => {
                EventAction::OrderSurchargeApplied(OrderSurchargeAppliedApplier)
            }
            EventPayload::OrderNoteAdded { .. } => {
                EventAction::OrderNoteAdded(OrderNoteAddedApplier)
            }
        }
    }
}
