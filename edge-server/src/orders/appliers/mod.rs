//! Event applier implementations
//!
//! Each applier implements the `EventApplier` trait and handles
//! one specific event type. Appliers are PURE functions.

use crate::orders::traits::EventApplier;
use shared::order::{EventPayload, OrderEvent, OrderSnapshot};

mod item_comped;
mod item_modified;
mod item_removed;
mod item_uncomped;
mod items_added;
mod member_linked;
mod member_unlinked;
mod order_adjustment_applied;
mod order_completed;
mod order_info_updated;
mod order_moved;
mod order_note_added;
mod order_split;
mod order_voided;
mod orders_merged;
mod payment_added;
mod payment_cancelled;
mod rule_skip_toggled;
mod stamp_redeemed;
mod stamp_redemption_cancelled;
mod table_opened;

pub use item_comped::ItemCompedApplier;
pub use item_modified::ItemModifiedApplier;
pub use item_removed::ItemRemovedApplier;
pub use item_uncomped::ItemUncompedApplier;
pub use items_added::ItemsAddedApplier;
pub use member_linked::MemberLinkedApplier;
pub use member_unlinked::MemberUnlinkedApplier;
pub use order_adjustment_applied::{OrderDiscountAppliedApplier, OrderSurchargeAppliedApplier};
pub use order_completed::OrderCompletedApplier;
pub use order_info_updated::OrderInfoUpdatedApplier;
pub use order_moved::OrderMovedApplier;
pub use order_note_added::OrderNoteAddedApplier;
pub use order_split::{
    AaSplitCancelledApplier, AaSplitPaidApplier, AaSplitStartedApplier, AmountSplitApplier,
    ItemSplitApplier,
};
pub use order_voided::OrderVoidedApplier;
pub use orders_merged::{OrderMergedApplier, OrderMergedOutApplier};
pub use payment_added::PaymentAddedApplier;
pub use payment_cancelled::PaymentCancelledApplier;
pub use rule_skip_toggled::RuleSkipToggledApplier;
pub use stamp_redeemed::StampRedeemedApplier;
pub use stamp_redemption_cancelled::StampRedemptionCancelledApplier;
pub use table_opened::TableOpenedApplier;

/// EventAction enum - dispatches to concrete applier implementations
pub enum EventAction {
    TableOpened(TableOpenedApplier),
    ItemsAdded(ItemsAddedApplier),
    ItemModified(ItemModifiedApplier),
    ItemRemoved(ItemRemovedApplier),
    ItemComped(ItemCompedApplier),
    ItemUncomped(ItemUncompedApplier),
    PaymentAdded(PaymentAddedApplier),
    PaymentCancelled(PaymentCancelledApplier),
    OrderCompleted(OrderCompletedApplier),
    OrderInfoUpdated(OrderInfoUpdatedApplier),
    OrderMoved(OrderMovedApplier),
    OrderVoided(OrderVoidedApplier),
    OrderMerged(OrderMergedApplier),
    OrderMergedOut(OrderMergedOutApplier),
    ItemSplit(ItemSplitApplier),
    AmountSplit(AmountSplitApplier),
    AaSplitStarted(AaSplitStartedApplier),
    AaSplitPaid(AaSplitPaidApplier),
    AaSplitCancelled(AaSplitCancelledApplier),
    RuleSkipToggled(RuleSkipToggledApplier),
    OrderDiscountApplied(OrderDiscountAppliedApplier),
    OrderSurchargeApplied(OrderSurchargeAppliedApplier),
    OrderNoteAdded(OrderNoteAddedApplier),
    MemberLinked(MemberLinkedApplier),
    MemberUnlinked(MemberUnlinkedApplier),
    StampRedeemed(StampRedeemedApplier),
    StampRedemptionCancelled(StampRedemptionCancelledApplier),
    /// Record-only events: persisted for timeline display, no snapshot mutation
    RecordOnly,
}

/// Manual implementation of EventApplier for EventAction
impl EventApplier for EventAction {
    fn apply(&self, snapshot: &mut OrderSnapshot, event: &OrderEvent) {
        match self {
            EventAction::TableOpened(applier) => applier.apply(snapshot, event),
            EventAction::ItemsAdded(applier) => applier.apply(snapshot, event),
            EventAction::ItemModified(applier) => applier.apply(snapshot, event),
            EventAction::ItemRemoved(applier) => applier.apply(snapshot, event),
            EventAction::ItemComped(applier) => applier.apply(snapshot, event),
            EventAction::ItemUncomped(applier) => applier.apply(snapshot, event),
            EventAction::PaymentAdded(applier) => applier.apply(snapshot, event),
            EventAction::PaymentCancelled(applier) => applier.apply(snapshot, event),
            EventAction::OrderCompleted(applier) => applier.apply(snapshot, event),
            EventAction::OrderInfoUpdated(applier) => applier.apply(snapshot, event),
            EventAction::OrderMoved(applier) => applier.apply(snapshot, event),
            EventAction::OrderVoided(applier) => applier.apply(snapshot, event),
            EventAction::OrderMerged(applier) => applier.apply(snapshot, event),
            EventAction::OrderMergedOut(applier) => applier.apply(snapshot, event),
            EventAction::ItemSplit(applier) => applier.apply(snapshot, event),
            EventAction::AmountSplit(applier) => applier.apply(snapshot, event),
            EventAction::AaSplitStarted(applier) => applier.apply(snapshot, event),
            EventAction::AaSplitPaid(applier) => applier.apply(snapshot, event),
            EventAction::AaSplitCancelled(applier) => applier.apply(snapshot, event),
            EventAction::RuleSkipToggled(applier) => applier.apply(snapshot, event),
            EventAction::OrderDiscountApplied(applier) => applier.apply(snapshot, event),
            EventAction::OrderSurchargeApplied(applier) => applier.apply(snapshot, event),
            EventAction::OrderNoteAdded(applier) => applier.apply(snapshot, event),
            EventAction::MemberLinked(applier) => applier.apply(snapshot, event),
            EventAction::MemberUnlinked(applier) => applier.apply(snapshot, event),
            EventAction::StampRedeemed(applier) => applier.apply(snapshot, event),
            EventAction::StampRedemptionCancelled(applier) => applier.apply(snapshot, event),
            EventAction::RecordOnly => {}
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
            EventPayload::ItemComped { .. } => EventAction::ItemComped(ItemCompedApplier),
            EventPayload::ItemUncomped { .. } => EventAction::ItemUncomped(ItemUncompedApplier),
            EventPayload::OrderMerged { .. } => EventAction::OrderMerged(OrderMergedApplier),
            EventPayload::OrderMergedOut { .. } => {
                EventAction::OrderMergedOut(OrderMergedOutApplier)
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
            EventPayload::OrderDiscountApplied { .. } => {
                EventAction::OrderDiscountApplied(OrderDiscountAppliedApplier)
            }
            EventPayload::OrderSurchargeApplied { .. } => {
                EventAction::OrderSurchargeApplied(OrderSurchargeAppliedApplier)
            }
            EventPayload::OrderNoteAdded { .. } => {
                EventAction::OrderNoteAdded(OrderNoteAddedApplier)
            }
            // Record-only events: persisted for timeline, no snapshot mutation
            EventPayload::OrderMovedOut { .. } | EventPayload::TableReassigned { .. } => {
                EventAction::RecordOnly
            }
            // Member events
            EventPayload::MemberLinked { .. } => EventAction::MemberLinked(MemberLinkedApplier),
            EventPayload::MemberUnlinked { .. } => {
                EventAction::MemberUnlinked(MemberUnlinkedApplier)
            }
            EventPayload::StampRedeemed { .. } => EventAction::StampRedeemed(StampRedeemedApplier),
            EventPayload::StampRedemptionCancelled { .. } => {
                EventAction::StampRedemptionCancelled(StampRedemptionCancelledApplier)
            }
        }
    }
}
