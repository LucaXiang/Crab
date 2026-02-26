//! Canonical binary serialization for deterministic hashing.
//!
//! Provides a `CanonicalHash` trait that produces stable, platform-independent
//! binary representations of order types. This is used for hash chain integrity
//! verification, decoupled from serde serialization.

use super::applied_mg_rule::AppliedMgRule;
use super::applied_rule::AppliedRule;
use super::event::{EventPayload, MgItemDiscount, OrderEventType};
use super::snapshot::OrderStatus;
use super::types::{
    CartItemSnapshot, CompRecord, ItemChanges, ItemModificationResult, ItemOption, LossReason,
    PaymentRecord, PaymentSummaryItem, ServiceType, SpecificationInfo, SplitItem, SplitType,
    StampRedemptionState, VoidType,
};
use crate::models::price_rule::{AdjustmentType, ProductScope, RuleType};

/// Trait for producing deterministic binary representations.
///
/// Implementations must be stable across serde format changes —
/// field order is fixed by source declaration, not by serialization.
pub trait CanonicalHash {
    fn canonical_bytes(&self, buf: &mut Vec<u8>);
}

// ============================================================================
// Helper functions
// ============================================================================

#[inline]
pub(crate) fn write_sep(buf: &mut Vec<u8>) {
    buf.push(0x00);
}

#[inline]
pub(crate) fn write_i64(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
#[allow(dead_code)]
pub(crate) fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
pub(crate) fn write_i32(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
pub(crate) fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
pub(crate) fn write_f64(buf: &mut Vec<u8>, v: f64) {
    buf.extend_from_slice(&v.to_bits().to_le_bytes());
}

#[inline]
pub(crate) fn write_bool(buf: &mut Vec<u8>, v: bool) {
    buf.push(if v { 0x01 } else { 0x00 });
}

#[inline]
pub(crate) fn write_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

#[inline]
pub(crate) fn write_tag(buf: &mut Vec<u8>, tag: &[u8]) {
    buf.extend_from_slice(tag);
}

#[inline]
pub(crate) fn write_opt<T: CanonicalHash>(buf: &mut Vec<u8>, opt: &Option<T>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            v.canonical_bytes(buf);
        }
    }
}

#[inline]
pub(crate) fn write_opt_i64(buf: &mut Vec<u8>, opt: Option<i64>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_i64(buf, v);
        }
    }
}

#[inline]
pub(crate) fn write_opt_i32(buf: &mut Vec<u8>, opt: Option<i32>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_i32(buf, v);
        }
    }
}

#[inline]
pub(crate) fn write_opt_u32(buf: &mut Vec<u8>, opt: Option<u32>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_u32(buf, v);
        }
    }
}

#[inline]
pub(crate) fn write_opt_f64(buf: &mut Vec<u8>, opt: Option<f64>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_f64(buf, v);
        }
    }
}

#[inline]
pub(crate) fn write_opt_str(buf: &mut Vec<u8>, opt: &Option<String>) {
    match opt {
        None => buf.push(0x00),
        Some(s) => {
            buf.push(0x01);
            write_str(buf, s);
        }
    }
}

#[inline]
pub(crate) fn write_opt_bool(buf: &mut Vec<u8>, opt: Option<bool>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_bool(buf, v);
        }
    }
}

#[inline]
pub(crate) fn write_vec<T: CanonicalHash>(buf: &mut Vec<u8>, items: &[T]) {
    write_u32(buf, items.len() as u32);
    for item in items {
        item.canonical_bytes(buf);
    }
}

#[inline]
pub(crate) fn write_opt_vec<T: CanonicalHash>(buf: &mut Vec<u8>, opt: &Option<Vec<T>>) {
    match opt {
        None => buf.push(0x00),
        Some(items) => {
            buf.push(0x01);
            write_vec(buf, items);
        }
    }
}

#[inline]
pub(crate) fn write_btreemap_str_i32(
    buf: &mut Vec<u8>,
    map: &std::collections::BTreeMap<String, i32>,
) {
    write_u32(buf, map.len() as u32);
    // BTreeMap iterates in key order — deterministic
    for (k, v) in map {
        write_str(buf, k);
        write_i32(buf, *v);
    }
}

impl CanonicalHash for String {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, self);
    }
}

// ============================================================================
// Enum implementations
// ============================================================================

impl CanonicalHash for OrderStatus {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            OrderStatus::Active => write_tag(buf, b"ACTIVE"),
            OrderStatus::Completed => write_tag(buf, b"COMPLETED"),
            OrderStatus::Void => write_tag(buf, b"VOID"),
            OrderStatus::Merged => write_tag(buf, b"MERGED"),
        }
    }
}

impl CanonicalHash for OrderEventType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            OrderEventType::TableOpened => write_tag(buf, b"TABLE_OPENED"),
            OrderEventType::OrderCompleted => write_tag(buf, b"ORDER_COMPLETED"),
            OrderEventType::OrderVoided => write_tag(buf, b"ORDER_VOIDED"),
            OrderEventType::ItemsAdded => write_tag(buf, b"ITEMS_ADDED"),
            OrderEventType::ItemModified => write_tag(buf, b"ITEM_MODIFIED"),
            OrderEventType::ItemRemoved => write_tag(buf, b"ITEM_REMOVED"),
            OrderEventType::ItemComped => write_tag(buf, b"ITEM_COMPED"),
            OrderEventType::ItemUncomped => write_tag(buf, b"ITEM_UNCOMPED"),
            OrderEventType::PaymentAdded => write_tag(buf, b"PAYMENT_ADDED"),
            OrderEventType::PaymentCancelled => write_tag(buf, b"PAYMENT_CANCELLED"),
            OrderEventType::ItemSplit => write_tag(buf, b"ITEM_SPLIT"),
            OrderEventType::AmountSplit => write_tag(buf, b"AMOUNT_SPLIT"),
            OrderEventType::AaSplitStarted => write_tag(buf, b"AA_SPLIT_STARTED"),
            OrderEventType::AaSplitPaid => write_tag(buf, b"AA_SPLIT_PAID"),
            OrderEventType::AaSplitCancelled => write_tag(buf, b"AA_SPLIT_CANCELLED"),
            OrderEventType::OrderMoved => write_tag(buf, b"ORDER_MOVED"),
            OrderEventType::OrderMovedOut => write_tag(buf, b"ORDER_MOVED_OUT"),
            OrderEventType::OrderMerged => write_tag(buf, b"ORDER_MERGED"),
            OrderEventType::OrderMergedOut => write_tag(buf, b"ORDER_MERGED_OUT"),
            OrderEventType::TableReassigned => write_tag(buf, b"TABLE_REASSIGNED"),
            OrderEventType::OrderInfoUpdated => write_tag(buf, b"ORDER_INFO_UPDATED"),
            OrderEventType::RuleSkipToggled => write_tag(buf, b"RULE_SKIP_TOGGLED"),
            OrderEventType::OrderDiscountApplied => write_tag(buf, b"ORDER_DISCOUNT_APPLIED"),
            OrderEventType::OrderSurchargeApplied => write_tag(buf, b"ORDER_SURCHARGE_APPLIED"),
            OrderEventType::OrderNoteAdded => write_tag(buf, b"ORDER_NOTE_ADDED"),
            OrderEventType::MemberLinked => write_tag(buf, b"MEMBER_LINKED"),
            OrderEventType::MemberUnlinked => write_tag(buf, b"MEMBER_UNLINKED"),
            OrderEventType::StampRedeemed => write_tag(buf, b"STAMP_REDEEMED"),
            OrderEventType::StampRedemptionCancelled => {
                write_tag(buf, b"STAMP_REDEMPTION_CANCELLED")
            }
        }
    }
}

impl CanonicalHash for VoidType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            VoidType::Cancelled => write_tag(buf, b"CANCELLED"),
            VoidType::LossSettled => write_tag(buf, b"LOSS_SETTLED"),
        }
    }
}

impl CanonicalHash for LossReason {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            LossReason::CustomerFled => write_tag(buf, b"CUSTOMER_FLED"),
            LossReason::RefusedToPay => write_tag(buf, b"REFUSED_TO_PAY"),
            LossReason::Other => write_tag(buf, b"OTHER"),
        }
    }
}

impl CanonicalHash for ServiceType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            ServiceType::DineIn => write_tag(buf, b"DINE_IN"),
            ServiceType::Takeout => write_tag(buf, b"TAKEOUT"),
        }
    }
}

impl CanonicalHash for SplitType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            SplitType::ItemSplit => write_tag(buf, b"ITEM_SPLIT"),
            SplitType::AmountSplit => write_tag(buf, b"AMOUNT_SPLIT"),
            SplitType::AaSplit => write_tag(buf, b"AA_SPLIT"),
        }
    }
}

impl CanonicalHash for RuleType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            RuleType::Discount => write_tag(buf, b"DISCOUNT"),
            RuleType::Surcharge => write_tag(buf, b"SURCHARGE"),
        }
    }
}

impl CanonicalHash for AdjustmentType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            AdjustmentType::Percentage => write_tag(buf, b"PERCENTAGE"),
            AdjustmentType::FixedAmount => write_tag(buf, b"FIXED_AMOUNT"),
        }
    }
}

impl CanonicalHash for ProductScope {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            ProductScope::Global => write_tag(buf, b"GLOBAL"),
            ProductScope::Category => write_tag(buf, b"CATEGORY"),
            ProductScope::Tag => write_tag(buf, b"TAG"),
            ProductScope::Product => write_tag(buf, b"PRODUCT"),
        }
    }
}

// ============================================================================
// Small struct implementations
// ============================================================================

impl CanonicalHash for ItemOption {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.attribute_id);
        write_str(buf, &self.attribute_name);
        write_i64(buf, self.option_id);
        write_str(buf, &self.option_name);
        write_opt_f64(buf, self.price_modifier);
        write_i32(buf, self.quantity);
        write_opt_str(buf, &self.receipt_name);
        write_opt_str(buf, &self.kitchen_print_name);
        write_bool(buf, self.show_on_receipt);
        write_bool(buf, self.show_on_kitchen_print);
    }
}

impl CanonicalHash for SpecificationInfo {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.id);
        write_str(buf, &self.name);
        write_opt_str(buf, &self.receipt_name);
        write_opt_f64(buf, self.price);
        write_bool(buf, self.is_multi_spec);
    }
}

impl CanonicalHash for AppliedRule {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.rule_id);
        write_str(buf, &self.name);
        write_opt_str(buf, &self.receipt_name);
        self.rule_type.canonical_bytes(buf);
        self.adjustment_type.canonical_bytes(buf);
        self.product_scope.canonical_bytes(buf);
        write_str(buf, &self.zone_scope);
        write_f64(buf, self.adjustment_value);
        write_f64(buf, self.calculated_amount);
        write_bool(buf, self.is_stackable);
        write_bool(buf, self.is_exclusive);
        write_bool(buf, self.skipped);
    }
}

impl CanonicalHash for AppliedMgRule {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.rule_id);
        write_str(buf, &self.name);
        write_opt_str(buf, &self.receipt_name);
        self.product_scope.canonical_bytes(buf);
        self.adjustment_type.canonical_bytes(buf);
        write_f64(buf, self.adjustment_value);
        write_f64(buf, self.calculated_amount);
        write_bool(buf, self.skipped);
    }
}

impl CanonicalHash for SplitItem {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, &self.instance_id);
        write_str(buf, &self.name);
        write_i32(buf, self.quantity);
        write_f64(buf, self.unit_price);
    }
}

impl CanonicalHash for ItemChanges {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_opt_f64(buf, self.price);
        write_opt_i32(buf, self.quantity);
        write_opt_f64(buf, self.manual_discount_percent);
        write_opt_str(buf, &self.note);
        write_opt_vec(buf, &self.selected_options);
        write_opt(buf, &self.selected_specification);
    }
}

impl CanonicalHash for ItemModificationResult {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, &self.instance_id);
        write_i32(buf, self.quantity);
        write_f64(buf, self.price);
        write_opt_f64(buf, self.manual_discount_percent);
        write_str(buf, &self.action);
    }
}

impl CanonicalHash for PaymentSummaryItem {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, &self.method);
        write_f64(buf, self.amount);
    }
}

impl CanonicalHash for MgItemDiscount {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, &self.instance_id);
        write_vec(buf, &self.applied_mg_rules);
    }
}

// ============================================================================
// Large struct implementations
// ============================================================================

impl CanonicalHash for CartItemSnapshot {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.id);
        write_str(buf, &self.instance_id);
        write_str(buf, &self.name);
        write_f64(buf, self.price);
        write_f64(buf, self.original_price);
        write_i32(buf, self.quantity);
        write_i32(buf, self.unpaid_quantity);
        write_opt_vec(buf, &self.selected_options);
        write_opt(buf, &self.selected_specification);
        write_opt_f64(buf, self.manual_discount_percent);
        write_f64(buf, self.rule_discount_amount);
        write_f64(buf, self.rule_surcharge_amount);
        write_vec(buf, &self.applied_rules);
        write_vec(buf, &self.applied_mg_rules);
        write_f64(buf, self.mg_discount_amount);
        write_f64(buf, self.unit_price);
        write_f64(buf, self.line_total);
        write_f64(buf, self.tax);
        write_i32(buf, self.tax_rate);
        write_opt_str(buf, &self.note);
        write_opt_i64(buf, self.authorizer_id);
        write_opt_str(buf, &self.authorizer_name);
        write_opt_i64(buf, self.category_id);
        write_opt_str(buf, &self.category_name);
        write_bool(buf, self.is_comped);
    }
}

impl CanonicalHash for PaymentRecord {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, &self.payment_id);
        write_str(buf, &self.method);
        write_f64(buf, self.amount);
        write_opt_f64(buf, self.tendered);
        write_opt_f64(buf, self.change);
        write_opt_str(buf, &self.note);
        write_i64(buf, self.timestamp);
        write_bool(buf, self.cancelled);
        write_opt_str(buf, &self.cancel_reason);
        write_opt_vec(buf, &self.split_items);
        write_opt_i32(buf, self.aa_shares);
        write_opt(buf, &self.split_type);
    }
}

impl CanonicalHash for CompRecord {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, &self.comp_id);
        write_str(buf, &self.instance_id);
        write_str(buf, &self.source_instance_id);
        write_str(buf, &self.item_name);
        write_i32(buf, self.quantity);
        write_f64(buf, self.original_price);
        write_str(buf, &self.reason);
        write_i64(buf, self.authorizer_id);
        write_str(buf, &self.authorizer_name);
        write_i64(buf, self.timestamp);
    }
}

impl CanonicalHash for StampRedemptionState {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.stamp_activity_id);
        write_str(buf, &self.reward_instance_id);
        write_bool(buf, self.is_comp_existing);
        write_opt_str(buf, &self.comp_source_instance_id);
    }
}

// ============================================================================
// EventPayload implementation
// ============================================================================

impl CanonicalHash for EventPayload {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            EventPayload::TableOpened {
                table_id,
                table_name,
                zone_id,
                zone_name,
                guest_count,
                is_retail,
                queue_number,
                receipt_number,
            } => {
                write_tag(buf, b"TABLE_OPENED");
                write_sep(buf);
                write_opt_i64(buf, *table_id);
                write_opt_str(buf, table_name);
                write_opt_i64(buf, *zone_id);
                write_opt_str(buf, zone_name);
                write_i32(buf, *guest_count);
                write_bool(buf, *is_retail);
                write_opt_u32(buf, *queue_number);
                write_str(buf, receipt_number);
            }

            EventPayload::OrderCompleted {
                receipt_number,
                service_type,
                final_total,
                payment_summary,
            } => {
                write_tag(buf, b"ORDER_COMPLETED");
                write_sep(buf);
                write_str(buf, receipt_number);
                write_opt(buf, service_type);
                write_f64(buf, *final_total);
                write_vec(buf, payment_summary);
            }

            EventPayload::OrderVoided {
                void_type,
                loss_reason,
                loss_amount,
                note,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ORDER_VOIDED");
                write_sep(buf);
                void_type.canonical_bytes(buf);
                write_opt(buf, loss_reason);
                write_opt_f64(buf, *loss_amount);
                write_opt_str(buf, note);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::ItemsAdded { items } => {
                write_tag(buf, b"ITEMS_ADDED");
                write_sep(buf);
                write_vec(buf, items);
            }

            EventPayload::ItemModified {
                operation,
                source,
                affected_quantity,
                changes,
                previous_values,
                results,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ITEM_MODIFIED");
                write_sep(buf);
                write_str(buf, operation);
                source.canonical_bytes(buf);
                write_i32(buf, *affected_quantity);
                changes.canonical_bytes(buf);
                previous_values.canonical_bytes(buf);
                write_vec(buf, results);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::ItemRemoved {
                instance_id,
                item_name,
                quantity,
                reason,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ITEM_REMOVED");
                write_sep(buf);
                write_str(buf, instance_id);
                write_str(buf, item_name);
                write_opt_i32(buf, *quantity);
                write_opt_str(buf, reason);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::ItemComped {
                instance_id,
                source_instance_id,
                item_name,
                quantity,
                original_price,
                reason,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ITEM_COMPED");
                write_sep(buf);
                write_str(buf, instance_id);
                write_str(buf, source_instance_id);
                write_str(buf, item_name);
                write_i32(buf, *quantity);
                write_f64(buf, *original_price);
                write_str(buf, reason);
                write_i64(buf, *authorizer_id);
                write_str(buf, authorizer_name);
            }

            EventPayload::ItemUncomped {
                instance_id,
                item_name,
                restored_price,
                merged_into,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ITEM_UNCOMPED");
                write_sep(buf);
                write_str(buf, instance_id);
                write_str(buf, item_name);
                write_f64(buf, *restored_price);
                write_opt_str(buf, merged_into);
                write_i64(buf, *authorizer_id);
                write_str(buf, authorizer_name);
            }

            EventPayload::PaymentAdded {
                payment_id,
                method,
                amount,
                tendered,
                change,
                note,
            } => {
                write_tag(buf, b"PAYMENT_ADDED");
                write_sep(buf);
                write_str(buf, payment_id);
                write_str(buf, method);
                write_f64(buf, *amount);
                write_opt_f64(buf, *tendered);
                write_opt_f64(buf, *change);
                write_opt_str(buf, note);
            }

            EventPayload::PaymentCancelled {
                payment_id,
                method,
                amount,
                reason,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"PAYMENT_CANCELLED");
                write_sep(buf);
                write_str(buf, payment_id);
                write_str(buf, method);
                write_f64(buf, *amount);
                write_opt_str(buf, reason);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::ItemSplit {
                payment_id,
                split_amount,
                payment_method,
                items,
                tendered,
                change,
            } => {
                write_tag(buf, b"ITEM_SPLIT");
                write_sep(buf);
                write_str(buf, payment_id);
                write_f64(buf, *split_amount);
                write_str(buf, payment_method);
                write_vec(buf, items);
                write_opt_f64(buf, *tendered);
                write_opt_f64(buf, *change);
            }

            EventPayload::AmountSplit {
                payment_id,
                split_amount,
                payment_method,
                tendered,
                change,
            } => {
                write_tag(buf, b"AMOUNT_SPLIT");
                write_sep(buf);
                write_str(buf, payment_id);
                write_f64(buf, *split_amount);
                write_str(buf, payment_method);
                write_opt_f64(buf, *tendered);
                write_opt_f64(buf, *change);
            }

            EventPayload::AaSplitStarted {
                total_shares,
                per_share_amount,
                order_total,
            } => {
                write_tag(buf, b"AA_SPLIT_STARTED");
                write_sep(buf);
                write_i32(buf, *total_shares);
                write_f64(buf, *per_share_amount);
                write_f64(buf, *order_total);
            }

            EventPayload::AaSplitPaid {
                payment_id,
                shares,
                amount,
                payment_method,
                progress_paid,
                progress_total,
                tendered,
                change,
            } => {
                write_tag(buf, b"AA_SPLIT_PAID");
                write_sep(buf);
                write_str(buf, payment_id);
                write_i32(buf, *shares);
                write_f64(buf, *amount);
                write_str(buf, payment_method);
                write_i32(buf, *progress_paid);
                write_i32(buf, *progress_total);
                write_opt_f64(buf, *tendered);
                write_opt_f64(buf, *change);
            }

            EventPayload::AaSplitCancelled { total_shares } => {
                write_tag(buf, b"AA_SPLIT_CANCELLED");
                write_sep(buf);
                write_i32(buf, *total_shares);
            }

            EventPayload::OrderMoved {
                source_table_id,
                source_table_name,
                target_table_id,
                target_table_name,
                target_zone_id,
                target_zone_name,
                items,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ORDER_MOVED");
                write_sep(buf);
                write_i64(buf, *source_table_id);
                write_str(buf, source_table_name);
                write_i64(buf, *target_table_id);
                write_str(buf, target_table_name);
                write_opt_i64(buf, *target_zone_id);
                write_opt_str(buf, target_zone_name);
                write_vec(buf, items);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::OrderMovedOut {
                target_table_id,
                target_table_name,
                reason,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ORDER_MOVED_OUT");
                write_sep(buf);
                write_i64(buf, *target_table_id);
                write_str(buf, target_table_name);
                write_opt_str(buf, reason);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::OrderMerged {
                source_table_id,
                source_table_name,
                items,
                payments,
                paid_item_quantities,
                paid_amount,
                has_amount_split,
                aa_total_shares,
                aa_paid_shares,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ORDER_MERGED");
                write_sep(buf);
                write_i64(buf, *source_table_id);
                write_str(buf, source_table_name);
                write_vec(buf, items);
                write_vec(buf, payments);
                write_btreemap_str_i32(buf, paid_item_quantities);
                write_f64(buf, *paid_amount);
                write_bool(buf, *has_amount_split);
                write_opt_i32(buf, *aa_total_shares);
                write_i32(buf, *aa_paid_shares);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::OrderMergedOut {
                target_table_id,
                target_table_name,
                reason,
                authorizer_id,
                authorizer_name,
            } => {
                write_tag(buf, b"ORDER_MERGED_OUT");
                write_sep(buf);
                write_i64(buf, *target_table_id);
                write_str(buf, target_table_name);
                write_opt_str(buf, reason);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            EventPayload::TableReassigned {
                source_table_id,
                source_table_name,
                target_table_id,
                target_table_name,
                target_zone_name,
                original_start_time,
                items,
            } => {
                write_tag(buf, b"TABLE_REASSIGNED");
                write_sep(buf);
                write_i64(buf, *source_table_id);
                write_str(buf, source_table_name);
                write_i64(buf, *target_table_id);
                write_str(buf, target_table_name);
                write_opt_str(buf, target_zone_name);
                write_i64(buf, *original_start_time);
                write_vec(buf, items);
            }

            EventPayload::OrderInfoUpdated {
                guest_count,
                table_name,
                is_pre_payment,
            } => {
                write_tag(buf, b"ORDER_INFO_UPDATED");
                write_sep(buf);
                write_opt_i32(buf, *guest_count);
                write_opt_str(buf, table_name);
                write_opt_bool(buf, *is_pre_payment);
            }

            EventPayload::RuleSkipToggled {
                rule_id,
                rule_name,
                skipped,
            } => {
                write_tag(buf, b"RULE_SKIP_TOGGLED");
                write_sep(buf);
                write_i64(buf, *rule_id);
                write_str(buf, rule_name);
                write_bool(buf, *skipped);
            }

            EventPayload::OrderDiscountApplied {
                discount_percent,
                discount_fixed,
                previous_discount_percent,
                previous_discount_fixed,
                authorizer_id,
                authorizer_name,
                subtotal,
                discount,
                total,
            } => {
                write_tag(buf, b"ORDER_DISCOUNT_APPLIED");
                write_sep(buf);
                write_opt_f64(buf, *discount_percent);
                write_opt_f64(buf, *discount_fixed);
                write_opt_f64(buf, *previous_discount_percent);
                write_opt_f64(buf, *previous_discount_fixed);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
                write_f64(buf, *subtotal);
                write_f64(buf, *discount);
                write_f64(buf, *total);
            }

            EventPayload::OrderSurchargeApplied {
                surcharge_percent,
                surcharge_amount,
                previous_surcharge_percent,
                previous_surcharge_amount,
                authorizer_id,
                authorizer_name,
                subtotal,
                surcharge,
                total,
            } => {
                write_tag(buf, b"ORDER_SURCHARGE_APPLIED");
                write_sep(buf);
                write_opt_f64(buf, *surcharge_percent);
                write_opt_f64(buf, *surcharge_amount);
                write_opt_f64(buf, *previous_surcharge_percent);
                write_opt_f64(buf, *previous_surcharge_amount);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
                write_f64(buf, *subtotal);
                write_f64(buf, *surcharge);
                write_f64(buf, *total);
            }

            EventPayload::OrderNoteAdded {
                note,
                previous_note,
            } => {
                write_tag(buf, b"ORDER_NOTE_ADDED");
                write_sep(buf);
                write_str(buf, note);
                write_opt_str(buf, previous_note);
            }

            EventPayload::MemberLinked {
                member_id,
                member_name,
                marketing_group_id,
                marketing_group_name,
                mg_item_discounts,
            } => {
                write_tag(buf, b"MEMBER_LINKED");
                write_sep(buf);
                write_i64(buf, *member_id);
                write_str(buf, member_name);
                write_i64(buf, *marketing_group_id);
                write_str(buf, marketing_group_name);
                write_vec(buf, mg_item_discounts);
            }

            EventPayload::MemberUnlinked {
                previous_member_id,
                previous_member_name,
            } => {
                write_tag(buf, b"MEMBER_UNLINKED");
                write_sep(buf);
                write_i64(buf, *previous_member_id);
                write_str(buf, previous_member_name);
            }

            EventPayload::StampRedeemed {
                stamp_activity_id,
                stamp_activity_name,
                reward_instance_id,
                reward_strategy,
                product_id,
                product_name,
                original_price,
                quantity,
                tax_rate,
                category_id,
                category_name,
                comp_existing_instance_id,
            } => {
                write_tag(buf, b"STAMP_REDEEMED");
                write_sep(buf);
                write_i64(buf, *stamp_activity_id);
                write_str(buf, stamp_activity_name);
                write_str(buf, reward_instance_id);
                write_str(buf, reward_strategy);
                write_i64(buf, *product_id);
                write_str(buf, product_name);
                write_f64(buf, *original_price);
                write_i32(buf, *quantity);
                write_i32(buf, *tax_rate);
                write_opt_i64(buf, *category_id);
                write_opt_str(buf, category_name);
                write_opt_str(buf, comp_existing_instance_id);
            }

            EventPayload::StampRedemptionCancelled {
                stamp_activity_id,
                stamp_activity_name,
                reward_instance_id,
                is_comp_existing,
                comp_source_instance_id,
            } => {
                write_tag(buf, b"STAMP_REDEMPTION_CANCELLED");
                write_sep(buf);
                write_i64(buf, *stamp_activity_id);
                write_str(buf, stamp_activity_name);
                write_str(buf, reward_instance_id);
                write_bool(buf, *is_comp_existing);
                write_opt_str(buf, comp_source_instance_id);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    fn canonical_sha256(payload: &impl CanonicalHash) -> String {
        let mut buf = Vec::new();
        payload.canonical_bytes(&mut buf);
        format!("{:x}", Sha256::digest(&buf))
    }

    #[test]
    fn test_canonical_deterministic() {
        let payload = EventPayload::TableOpened {
            table_id: Some(1),
            table_name: Some("T1".to_string()),
            zone_id: Some(10),
            zone_name: Some("Main".to_string()),
            guest_count: 4,
            is_retail: false,
            queue_number: None,
            receipt_number: "R001".to_string(),
        };

        let h1 = canonical_sha256(&payload);
        let h2 = canonical_sha256(&payload);
        assert_eq!(h1, h2, "Same payload must produce identical hashes");
    }

    #[test]
    fn test_canonical_different_values_different_hash() {
        let p1 = EventPayload::TableOpened {
            table_id: Some(1),
            table_name: Some("T1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            queue_number: None,
            receipt_number: "R001".to_string(),
        };
        let p2 = EventPayload::TableOpened {
            table_id: Some(2),
            table_name: Some("T2".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            queue_number: None,
            receipt_number: "R001".to_string(),
        };

        assert_ne!(
            canonical_sha256(&p1),
            canonical_sha256(&p2),
            "Different values must produce different hashes"
        );
    }

    #[test]
    fn test_canonical_none_vs_some_different() {
        let p_none = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: 50.0,
            tendered: None,
            change: None,
            note: None,
        };
        let p_some = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: 50.0,
            tendered: Some(50.0),
            change: Some(0.0),
            note: None,
        };

        assert_ne!(
            canonical_sha256(&p_none),
            canonical_sha256(&p_some),
            "None vs Some must produce different hashes"
        );
    }

    #[test]
    fn test_canonical_roundtrip_stable() {
        let payload = EventPayload::ItemsAdded {
            items: vec![CartItemSnapshot {
                id: 42,
                instance_id: "inst-42".to_string(),
                name: "Burger".to_string(),
                price: 12.50,
                original_price: 15.00,
                quantity: 2,
                unpaid_quantity: 2,
                selected_options: Some(vec![ItemOption {
                    attribute_id: 1,
                    attribute_name: "Size".to_string(),
                    option_id: 2,
                    option_name: "Large".to_string(),
                    price_modifier: Some(2.0),
                    quantity: 1,
                    receipt_name: None,
                    kitchen_print_name: None,
                    show_on_receipt: true,
                    show_on_kitchen_print: true,
                }]),
                selected_specification: None,
                manual_discount_percent: Some(10.0),
                rule_discount_amount: 1.5,
                rule_surcharge_amount: 0.0,
                applied_rules: vec![],
                applied_mg_rules: vec![],
                mg_discount_amount: 0.0,
                unit_price: 11.25,
                line_total: 22.50,
                tax: 4.73,
                tax_rate: 21,
                note: Some("no onions".to_string()),
                authorizer_id: None,
                authorizer_name: None,
                category_id: Some(5),
                category_name: Some("Burgers".to_string()),
                is_comped: false,
            }],
        };

        let hash_before = canonical_sha256(&payload);

        // Roundtrip through JSON
        let json = serde_json::to_string(&payload).unwrap();
        let restored: EventPayload = serde_json::from_str(&json).unwrap();

        let hash_after = canonical_sha256(&restored);
        assert_eq!(
            hash_before, hash_after,
            "Canonical hash must survive JSON roundtrip"
        );
    }

    #[test]
    fn test_canonical_f64_roundtrip_stable() {
        let payload = EventPayload::OrderCompleted {
            receipt_number: "R100".to_string(),
            service_type: Some(ServiceType::DineIn),
            final_total: 99.99,
            payment_summary: vec![PaymentSummaryItem {
                method: "card".to_string(),
                amount: 99.99,
            }],
        };

        let hash_before = canonical_sha256(&payload);

        let json = serde_json::to_string(&payload).unwrap();
        let restored: EventPayload = serde_json::from_str(&json).unwrap();

        let hash_after = canonical_sha256(&restored);
        assert_eq!(
            hash_before, hash_after,
            "f64 values must survive JSON roundtrip with identical canonical bytes"
        );
    }

    #[test]
    fn test_canonical_all_event_types_covered() {
        let all_types = [
            OrderEventType::TableOpened,
            OrderEventType::OrderCompleted,
            OrderEventType::OrderVoided,
            OrderEventType::ItemsAdded,
            OrderEventType::ItemModified,
            OrderEventType::ItemRemoved,
            OrderEventType::ItemComped,
            OrderEventType::ItemUncomped,
            OrderEventType::PaymentAdded,
            OrderEventType::PaymentCancelled,
            OrderEventType::ItemSplit,
            OrderEventType::AmountSplit,
            OrderEventType::AaSplitStarted,
            OrderEventType::AaSplitPaid,
            OrderEventType::AaSplitCancelled,
            OrderEventType::OrderMoved,
            OrderEventType::OrderMovedOut,
            OrderEventType::OrderMerged,
            OrderEventType::OrderMergedOut,
            OrderEventType::TableReassigned,
            OrderEventType::OrderInfoUpdated,
            OrderEventType::RuleSkipToggled,
            OrderEventType::OrderDiscountApplied,
            OrderEventType::OrderSurchargeApplied,
            OrderEventType::OrderNoteAdded,
            OrderEventType::MemberLinked,
            OrderEventType::MemberUnlinked,
            OrderEventType::StampRedeemed,
            OrderEventType::StampRedemptionCancelled,
        ];

        let mut hashes = std::collections::HashSet::new();
        for et in &all_types {
            let h = canonical_sha256(et);
            assert!(
                hashes.insert(h.clone()),
                "Duplicate hash for event type {:?}",
                et
            );
        }

        assert_eq!(
            hashes.len(),
            29,
            "Must cover all 29 OrderEventType variants"
        );
    }

    #[test]
    fn test_canonical_order_status_all_unique() {
        let statuses = [
            OrderStatus::Active,
            OrderStatus::Completed,
            OrderStatus::Void,
            OrderStatus::Merged,
        ];

        let mut hashes = std::collections::HashSet::new();
        for s in &statuses {
            let h = canonical_sha256(s);
            assert!(
                hashes.insert(h.clone()),
                "Duplicate hash for status {:?}",
                s
            );
        }
        assert_eq!(hashes.len(), 4);
    }

    #[test]
    fn test_golden_table_opened() {
        let payload = EventPayload::TableOpened {
            table_id: Some(1),
            table_name: Some("Mesa 1".to_string()),
            zone_id: Some(10),
            zone_name: Some("Terraza".to_string()),
            guest_count: 4,
            is_retail: false,
            queue_number: None,
            receipt_number: "R-20240101-001".to_string(),
        };

        let hash = canonical_sha256(&payload);
        // Golden value — if this changes, the canonical encoding has broken
        assert_eq!(
            hash, "ba53f6636491acd0a37b209c7b4bfdbac39563a2b6af14ca1b55b2a45ea76d82",
            "Golden hash mismatch — canonical encoding changed!"
        );
    }
}
