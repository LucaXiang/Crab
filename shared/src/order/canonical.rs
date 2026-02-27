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
pub fn write_sep(buf: &mut Vec<u8>) {
    buf.push(0x00);
}

#[inline]
pub fn write_i64(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn write_i32(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn write_f64(buf: &mut Vec<u8>, v: f64) {
    // Normalize -0.0 to 0.0 to ensure JSON roundtrip stability
    // (serde_json serializes -0.0 as "0" which deserializes to 0.0)
    let normalized = if v == 0.0 { 0.0_f64 } else { v };
    buf.extend_from_slice(&normalized.to_bits().to_le_bytes());
}

#[inline]
pub fn write_bool(buf: &mut Vec<u8>, v: bool) {
    buf.push(if v { 0x01 } else { 0x00 });
}

#[inline]
pub fn write_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

#[inline]
pub fn write_tag(buf: &mut Vec<u8>, tag: &[u8]) {
    buf.extend_from_slice(tag);
}

#[inline]
pub fn write_opt<T: CanonicalHash>(buf: &mut Vec<u8>, opt: &Option<T>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            v.canonical_bytes(buf);
        }
    }
}

#[inline]
pub fn write_opt_i64(buf: &mut Vec<u8>, opt: Option<i64>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_i64(buf, v);
        }
    }
}

#[inline]
pub fn write_opt_i32(buf: &mut Vec<u8>, opt: Option<i32>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_i32(buf, v);
        }
    }
}

#[inline]
pub fn write_opt_u32(buf: &mut Vec<u8>, opt: Option<u32>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_u32(buf, v);
        }
    }
}

#[inline]
pub fn write_opt_f64(buf: &mut Vec<u8>, opt: Option<f64>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_f64(buf, v);
        }
    }
}

#[inline]
pub fn write_opt_str(buf: &mut Vec<u8>, opt: &Option<String>) {
    match opt {
        None => buf.push(0x00),
        Some(s) => {
            buf.push(0x01);
            write_str(buf, s);
        }
    }
}

#[inline]
pub fn write_opt_bool(buf: &mut Vec<u8>, opt: Option<bool>) {
    match opt {
        None => buf.push(0x00),
        Some(v) => {
            buf.push(0x01);
            write_bool(buf, v);
        }
    }
}

#[inline]
pub fn write_vec<T: CanonicalHash>(buf: &mut Vec<u8>, items: &[T]) {
    write_u32(buf, items.len() as u32);
    for item in items {
        item.canonical_bytes(buf);
    }
}

#[inline]
pub fn write_opt_vec<T: CanonicalHash>(buf: &mut Vec<u8>, opt: &Option<Vec<T>>) {
    match opt {
        None => buf.push(0x00),
        Some(items) => {
            buf.push(0x01);
            write_vec(buf, items);
        }
    }
}

#[inline]
pub fn write_btreemap_str_i32(buf: &mut Vec<u8>, map: &std::collections::BTreeMap<String, i32>) {
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
// OrderEvent canonical hash — includes ALL event metadata fields
// ============================================================================

impl CanonicalHash for super::event::OrderEvent {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, &self.event_id);
        write_str(buf, &self.order_id);
        write_u64(buf, self.sequence);
        write_i64(buf, self.timestamp);
        write_i64(buf, self.operator_id);
        write_str(buf, &self.operator_name);
        write_str(buf, &self.command_id);
        write_opt_i64(buf, self.client_timestamp);
        self.event_type.canonical_bytes(buf);
        write_sep(buf);
        self.payload.canonical_bytes(buf);
    }
}

/// Compute the order chain hash linking orders together.
///
/// Hash = SHA256(prev_hash || order_id || receipt_number || status || last_event_hash)
/// All strings are length-prefixed for unambiguous boundary separation.
pub fn compute_order_chain_hash(
    prev_hash: &str,
    order_id: &str,
    receipt_number: &str,
    status: &OrderStatus,
    last_event_hash: &str,
) -> String {
    use sha2::{Digest, Sha256};

    let mut buf = Vec::with_capacity(256);
    write_str(&mut buf, prev_hash);
    write_str(&mut buf, order_id);
    write_str(&mut buf, receipt_number);
    status.canonical_bytes(&mut buf);
    write_str(&mut buf, last_event_hash);

    format!("{:x}", Sha256::digest(&buf))
}

/// Compute the event hash for tamper-proof verification.
///
/// Hash = SHA256(canonical_bytes(OrderEvent))
pub fn compute_event_chain_hash(event: &super::event::OrderEvent) -> String {
    use sha2::{Digest, Sha256};

    let mut buf = Vec::with_capacity(512);
    event.canonical_bytes(&mut buf);
    format!("{:x}", Sha256::digest(&buf))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;

    fn canonical_sha256(payload: &impl CanonicalHash) -> String {
        let mut buf = Vec::new();
        payload.canonical_bytes(&mut buf);
        format!("{:x}", Sha256::digest(&buf))
    }

    // ========================================================================
    // Helper: build a fully-populated CartItemSnapshot
    // ========================================================================

    fn full_cart_item() -> CartItemSnapshot {
        CartItemSnapshot {
            id: 42,
            instance_id: "inst-42".to_string(),
            name: "Paella Valenciana".to_string(),
            price: 12.50,
            original_price: 15.00,
            quantity: 2,
            unpaid_quantity: 1,
            selected_options: Some(vec![ItemOption {
                attribute_id: 1,
                attribute_name: "Size".to_string(),
                option_id: 2,
                option_name: "Large".to_string(),
                price_modifier: Some(2.0),
                quantity: 3,
                receipt_name: Some("LG".to_string()),
                kitchen_print_name: Some("LARGE".to_string()),
                show_on_receipt: true,
                show_on_kitchen_print: true,
            }]),
            selected_specification: Some(SpecificationInfo {
                id: 10,
                name: "Spicy".to_string(),
                receipt_name: Some("SPC".to_string()),
                price: Some(1.50),
                is_multi_spec: true,
            }),
            manual_discount_percent: Some(10.0),
            rule_discount_amount: 1.5,
            rule_surcharge_amount: 0.50,
            applied_rules: vec![AppliedRule {
                rule_id: 100,
                name: "Lunch Special".to_string(),
                receipt_name: Some("LUNCH".to_string()),
                rule_type: RuleType::Discount,
                adjustment_type: AdjustmentType::Percentage,
                product_scope: ProductScope::Global,
                zone_scope: "all".to_string(),
                adjustment_value: 10.0,
                calculated_amount: 1.5,
                is_stackable: true,
                is_exclusive: false,
                skipped: false,
            }],
            applied_mg_rules: vec![AppliedMgRule {
                rule_id: 200,
                name: "VIP Discount".to_string(),
                receipt_name: Some("VIP".to_string()),
                product_scope: ProductScope::Category,
                adjustment_type: AdjustmentType::FixedAmount,
                adjustment_value: 2.0,
                calculated_amount: 2.0,
                skipped: false,
            }],
            mg_discount_amount: 2.0,
            unit_price: 11.25,
            line_total: 22.50,
            tax: 4.73,
            tax_rate: 21,
            note: Some("sin cebolla".to_string()),
            authorizer_id: Some(99),
            authorizer_name: Some("Manager".to_string()),
            category_id: Some(5),
            category_name: Some("Arroces".to_string()),
            is_comped: false,
        }
    }

    fn full_payment_record() -> PaymentRecord {
        PaymentRecord {
            payment_id: "pay-1".to_string(),
            method: "cash".to_string(),
            amount: 50.0,
            tendered: Some(60.0),
            change: Some(10.0),
            note: Some("exact".to_string()),
            timestamp: 1700000000000,
            cancelled: false,
            cancel_reason: Some("test".to_string()),
            split_items: Some(vec![full_cart_item()]),
            aa_shares: Some(2),
            split_type: Some(SplitType::AaSplit),
        }
    }

    // ========================================================================
    // Helper: build all 29 EventPayload variants with full data
    // ========================================================================

    fn build_all_test_variants() -> Vec<(&'static str, EventPayload)> {
        vec![
            (
                "TableOpened",
                EventPayload::TableOpened {
                    table_id: Some(1),
                    table_name: Some("Mesa 1".to_string()),
                    zone_id: Some(10),
                    zone_name: Some("Terraza".to_string()),
                    guest_count: 4,
                    is_retail: false,
                    queue_number: Some(42),
                    receipt_number: "R-001".to_string(),
                },
            ),
            (
                "OrderCompleted",
                EventPayload::OrderCompleted {
                    receipt_number: "R-001".to_string(),
                    service_type: Some(ServiceType::DineIn),
                    final_total: 99.99,
                    payment_summary: vec![PaymentSummaryItem {
                        method: "card".to_string(),
                        amount: 99.99,
                    }],
                },
            ),
            (
                "OrderVoided",
                EventPayload::OrderVoided {
                    void_type: VoidType::LossSettled,
                    loss_reason: Some(LossReason::CustomerFled),
                    loss_amount: Some(45.50),
                    note: Some("customer left".to_string()),
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "ItemsAdded",
                EventPayload::ItemsAdded {
                    items: vec![full_cart_item()],
                },
            ),
            (
                "ItemModified",
                EventPayload::ItemModified {
                    operation: "change_quantity".to_string(),
                    source: Box::new(full_cart_item()),
                    affected_quantity: 1,
                    changes: Box::new(ItemChanges {
                        price: Some(10.0),
                        quantity: Some(3),
                        manual_discount_percent: Some(5.0),
                        note: Some("extra sauce".to_string()),
                        selected_options: Some(vec![ItemOption {
                            attribute_id: 1,
                            attribute_name: "Temp".to_string(),
                            option_id: 3,
                            option_name: "Hot".to_string(),
                            price_modifier: Some(0.5),
                            quantity: 1,
                            receipt_name: None,
                            kitchen_print_name: None,
                            show_on_receipt: true,
                            show_on_kitchen_print: false,
                        }]),
                        selected_specification: Some(SpecificationInfo {
                            id: 20,
                            name: "Medium".to_string(),
                            receipt_name: None,
                            price: Some(0.0),
                            is_multi_spec: false,
                        }),
                    }),
                    previous_values: Box::new(ItemChanges {
                        price: Some(12.50),
                        quantity: Some(2),
                        manual_discount_percent: None,
                        note: None,
                        selected_options: None,
                        selected_specification: None,
                    }),
                    results: vec![ItemModificationResult {
                        instance_id: "inst-42".to_string(),
                        quantity: 3,
                        price: 10.0,
                        manual_discount_percent: Some(5.0),
                        action: "UPDATED".to_string(),
                    }],
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "ItemRemoved",
                EventPayload::ItemRemoved {
                    instance_id: "inst-42".to_string(),
                    item_name: "Burger".to_string(),
                    quantity: Some(1),
                    reason: Some("wrong item".to_string()),
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "ItemComped",
                EventPayload::ItemComped {
                    instance_id: "inst-42::comp::abc".to_string(),
                    source_instance_id: "inst-42".to_string(),
                    item_name: "Burger".to_string(),
                    quantity: 1,
                    original_price: 12.50,
                    reason: "birthday gift".to_string(),
                    authorizer_id: 99,
                    authorizer_name: "Manager".to_string(),
                },
            ),
            (
                "ItemUncomped",
                EventPayload::ItemUncomped {
                    instance_id: "inst-42::comp::abc".to_string(),
                    item_name: "Burger".to_string(),
                    restored_price: 12.50,
                    merged_into: Some("inst-42".to_string()),
                    authorizer_id: 99,
                    authorizer_name: "Manager".to_string(),
                },
            ),
            (
                "PaymentAdded",
                EventPayload::PaymentAdded {
                    payment_id: "pay-1".to_string(),
                    method: "cash".to_string(),
                    amount: 50.0,
                    tendered: Some(60.0),
                    change: Some(10.0),
                    note: Some("exact change".to_string()),
                },
            ),
            (
                "PaymentCancelled",
                EventPayload::PaymentCancelled {
                    payment_id: "pay-1".to_string(),
                    method: "cash".to_string(),
                    amount: 50.0,
                    reason: Some("customer changed mind".to_string()),
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "ItemSplit",
                EventPayload::ItemSplit {
                    payment_id: "pay-split-1".to_string(),
                    split_amount: 25.0,
                    payment_method: "card".to_string(),
                    items: vec![SplitItem {
                        instance_id: "inst-42".to_string(),
                        name: "Burger".to_string(),
                        quantity: 1,
                        unit_price: 12.50,
                    }],
                    tendered: Some(25.0),
                    change: Some(0.0),
                },
            ),
            (
                "AmountSplit",
                EventPayload::AmountSplit {
                    payment_id: "pay-amount-1".to_string(),
                    split_amount: 33.33,
                    payment_method: "card".to_string(),
                    tendered: Some(35.0),
                    change: Some(1.67),
                },
            ),
            (
                "AaSplitStarted",
                EventPayload::AaSplitStarted {
                    total_shares: 3,
                    per_share_amount: 33.33,
                    order_total: 99.99,
                },
            ),
            (
                "AaSplitPaid",
                EventPayload::AaSplitPaid {
                    payment_id: "pay-aa-1".to_string(),
                    shares: 1,
                    amount: 33.33,
                    payment_method: "cash".to_string(),
                    progress_paid: 1,
                    progress_total: 3,
                    tendered: Some(40.0),
                    change: Some(6.67),
                },
            ),
            (
                "AaSplitCancelled",
                EventPayload::AaSplitCancelled { total_shares: 3 },
            ),
            (
                "OrderMoved",
                EventPayload::OrderMoved {
                    source_table_id: 1,
                    source_table_name: "Mesa 1".to_string(),
                    target_table_id: 5,
                    target_table_name: "Mesa 5".to_string(),
                    target_zone_id: Some(20),
                    target_zone_name: Some("Interior".to_string()),
                    items: vec![full_cart_item()],
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "OrderMovedOut",
                EventPayload::OrderMovedOut {
                    target_table_id: 5,
                    target_table_name: "Mesa 5".to_string(),
                    reason: Some("table change".to_string()),
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "OrderMerged",
                EventPayload::OrderMerged {
                    source_table_id: 2,
                    source_table_name: "Mesa 2".to_string(),
                    items: vec![full_cart_item()],
                    payments: vec![full_payment_record()],
                    paid_item_quantities: {
                        let mut m = BTreeMap::new();
                        m.insert("inst-42".to_string(), 1);
                        m.insert("inst-43".to_string(), 2);
                        m
                    },
                    paid_amount: 25.0,
                    has_amount_split: true,
                    aa_total_shares: Some(3),
                    aa_paid_shares: 1,
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "OrderMergedOut",
                EventPayload::OrderMergedOut {
                    target_table_id: 1,
                    target_table_name: "Mesa 1".to_string(),
                    reason: Some("merge tables".to_string()),
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                },
            ),
            (
                "TableReassigned",
                EventPayload::TableReassigned {
                    source_table_id: 1,
                    source_table_name: "Mesa 1".to_string(),
                    target_table_id: 5,
                    target_table_name: "Mesa 5".to_string(),
                    target_zone_name: Some("Terraza".to_string()),
                    original_start_time: 1700000000000,
                    items: vec![full_cart_item()],
                },
            ),
            (
                "OrderInfoUpdated",
                EventPayload::OrderInfoUpdated {
                    guest_count: Some(6),
                    table_name: Some("Mesa 10".to_string()),
                    is_pre_payment: Some(true),
                },
            ),
            (
                "RuleSkipToggled",
                EventPayload::RuleSkipToggled {
                    rule_id: 100,
                    rule_name: "Lunch Special".to_string(),
                    skipped: true,
                },
            ),
            (
                "OrderDiscountApplied",
                EventPayload::OrderDiscountApplied {
                    discount_percent: Some(15.0),
                    discount_fixed: Some(5.0),
                    previous_discount_percent: Some(10.0),
                    previous_discount_fixed: Some(3.0),
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                    subtotal: 100.0,
                    discount: 15.0,
                    total: 85.0,
                },
            ),
            (
                "OrderSurchargeApplied",
                EventPayload::OrderSurchargeApplied {
                    surcharge_percent: Some(10.0),
                    surcharge_amount: Some(5.0),
                    previous_surcharge_percent: Some(8.0),
                    previous_surcharge_amount: Some(4.0),
                    authorizer_id: Some(99),
                    authorizer_name: Some("Manager".to_string()),
                    subtotal: 100.0,
                    surcharge: 10.0,
                    total: 110.0,
                },
            ),
            (
                "OrderNoteAdded",
                EventPayload::OrderNoteAdded {
                    note: "VIP customer".to_string(),
                    previous_note: Some("regular".to_string()),
                },
            ),
            (
                "MemberLinked",
                EventPayload::MemberLinked {
                    member_id: 1001,
                    member_name: "Juan Garcia".to_string(),
                    marketing_group_id: 50,
                    marketing_group_name: "Gold Members".to_string(),
                    mg_item_discounts: vec![MgItemDiscount {
                        instance_id: "inst-42".to_string(),
                        applied_mg_rules: vec![AppliedMgRule {
                            rule_id: 200,
                            name: "VIP Discount".to_string(),
                            receipt_name: Some("VIP".to_string()),
                            product_scope: ProductScope::Global,
                            adjustment_type: AdjustmentType::Percentage,
                            adjustment_value: 10.0,
                            calculated_amount: 1.25,
                            skipped: false,
                        }],
                    }],
                },
            ),
            (
                "MemberUnlinked",
                EventPayload::MemberUnlinked {
                    previous_member_id: 1001,
                    previous_member_name: "Juan Garcia".to_string(),
                },
            ),
            (
                "StampRedeemed",
                EventPayload::StampRedeemed {
                    stamp_activity_id: 500,
                    stamp_activity_name: "Coffee Card".to_string(),
                    reward_instance_id: "reward-1".to_string(),
                    reward_strategy: "free_item".to_string(),
                    product_id: 42,
                    product_name: "Latte".to_string(),
                    original_price: 4.50,
                    quantity: 1,
                    tax_rate: 21,
                    category_id: Some(3),
                    category_name: Some("Drinks".to_string()),
                    comp_existing_instance_id: Some("inst-existing".to_string()),
                },
            ),
            (
                "StampRedemptionCancelled",
                EventPayload::StampRedemptionCancelled {
                    stamp_activity_id: 500,
                    stamp_activity_name: "Coffee Card".to_string(),
                    reward_instance_id: "reward-1".to_string(),
                    is_comp_existing: true,
                    comp_source_instance_id: Some("inst-existing".to_string()),
                },
            ),
        ]
    }

    // ========================================================================
    // A. Roundtrip tests for all 29 variants
    // ========================================================================

    fn assert_roundtrip_stable(name: &str, payload: &EventPayload) {
        let hash_before = canonical_sha256(payload);
        let json = serde_json::to_string(payload).unwrap();
        let restored: EventPayload = serde_json::from_str(&json).unwrap();
        let hash_after = canonical_sha256(&restored);
        assert_eq!(
            hash_before, hash_after,
            "roundtrip failed for variant: {}",
            name
        );
    }

    #[test]
    fn test_all_variants_roundtrip_stable() {
        let variants = build_all_test_variants();
        assert_eq!(
            variants.len(),
            29,
            "Must have test data for all 29 EventPayload variants"
        );
        for (name, payload) in &variants {
            assert_roundtrip_stable(name, payload);
        }
    }

    #[test]
    fn test_all_variants_produce_unique_hashes() {
        let variants = build_all_test_variants();
        let mut hashes = std::collections::HashSet::new();
        for (name, payload) in &variants {
            let h = canonical_sha256(payload);
            assert!(hashes.insert(h), "Duplicate hash for variant: {}", name);
        }
    }

    // ========================================================================
    // B. Boundary case tests
    // ========================================================================

    #[test]
    fn test_empty_string_vs_nonempty_string() {
        let p_empty = EventPayload::OrderNoteAdded {
            note: "".to_string(),
            previous_note: None,
        };
        let p_nonempty = EventPayload::OrderNoteAdded {
            note: "hello".to_string(),
            previous_note: None,
        };
        assert_ne!(
            canonical_sha256(&p_empty),
            canonical_sha256(&p_nonempty),
            "Empty vs non-empty string must differ"
        );
    }

    #[test]
    fn test_f64_zero_vs_negative_zero() {
        // -0.0 is normalized to 0.0 in write_f64 to ensure JSON roundtrip stability
        // (serde_json serializes -0.0 as "0" which deserializes to 0.0)
        let p_pos = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: 0.0,
            tendered: None,
            change: None,
            note: None,
        };
        let p_neg = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: -0.0,
            tendered: None,
            change: None,
            note: None,
        };
        // After normalization, 0.0 and -0.0 produce the same hash
        assert_eq!(
            canonical_sha256(&p_pos),
            canonical_sha256(&p_neg),
            "0.0 and -0.0 must produce equal hashes (normalization ensures JSON roundtrip stability)"
        );
    }

    #[test]
    fn test_f64_negative_zero_json_roundtrip() {
        // Verify that -0.0 survives JSON roundtrip (serde_json normalizes it to 0.0)
        let payload = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: -0.0,
            tendered: None,
            change: None,
            note: None,
        };
        let hash_before = canonical_sha256(&payload);
        let json = serde_json::to_string(&payload).unwrap();
        let roundtripped: EventPayload = serde_json::from_str(&json).unwrap();
        let hash_after = canonical_sha256(&roundtripped);
        assert_eq!(
            hash_before, hash_after,
            "-0.0 must produce stable hash after JSON roundtrip"
        );
    }

    #[test]
    fn test_f64_zero_roundtrip_stable() {
        // Crucially, 0.0 survives JSON roundtrip as 0.0 (not -0.0)
        let payload = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: 0.0,
            tendered: None,
            change: None,
            note: None,
        };
        assert_roundtrip_stable("PaymentAdded-zero", &payload);
    }

    #[test]
    fn test_f64_small_amounts() {
        // Common money edge cases
        for amount in [0.01, 0.001, 0.1, 1.0, 9.99, 99.99, 999.99, 0.0] {
            let payload = EventPayload::PaymentAdded {
                payment_id: "p1".to_string(),
                method: "cash".to_string(),
                amount,
                tendered: None,
                change: None,
                note: None,
            };
            assert_roundtrip_stable(&format!("PaymentAdded-{}", amount), &payload);
        }
    }

    #[test]
    fn test_empty_vec_vs_nonempty_vec() {
        let p_empty = EventPayload::ItemsAdded { items: vec![] };
        let p_nonempty = EventPayload::ItemsAdded {
            items: vec![full_cart_item()],
        };
        assert_ne!(
            canonical_sha256(&p_empty),
            canonical_sha256(&p_nonempty),
            "Empty vec vs non-empty vec must differ"
        );
    }

    #[test]
    fn test_empty_btreemap_vs_nonempty() {
        let p_empty = EventPayload::OrderMerged {
            source_table_id: 1,
            source_table_name: "T1".to_string(),
            items: vec![],
            payments: vec![],
            paid_item_quantities: BTreeMap::new(),
            paid_amount: 0.0,
            has_amount_split: false,
            aa_total_shares: None,
            aa_paid_shares: 0,
            authorizer_id: None,
            authorizer_name: None,
        };
        let p_nonempty = EventPayload::OrderMerged {
            source_table_id: 1,
            source_table_name: "T1".to_string(),
            items: vec![],
            payments: vec![],
            paid_item_quantities: {
                let mut m = BTreeMap::new();
                m.insert("inst-1".to_string(), 1);
                m
            },
            paid_amount: 0.0,
            has_amount_split: false,
            aa_total_shares: None,
            aa_paid_shares: 0,
            authorizer_id: None,
            authorizer_name: None,
        };
        assert_ne!(
            canonical_sha256(&p_empty),
            canonical_sha256(&p_nonempty),
            "Empty BTreeMap vs non-empty must differ"
        );
    }

    // ========================================================================
    // C. Golden tests for commonly used variants
    // ========================================================================

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
        assert_eq!(
            hash, "ba53f6636491acd0a37b209c7b4bfdbac39563a2b6af14ca1b55b2a45ea76d82",
            "Golden hash mismatch — canonical encoding changed!"
        );
    }

    #[test]
    fn test_golden_order_completed() {
        let payload = EventPayload::OrderCompleted {
            receipt_number: "R-20240101-001".to_string(),
            service_type: Some(ServiceType::DineIn),
            final_total: 85.50,
            payment_summary: vec![
                PaymentSummaryItem {
                    method: "cash".to_string(),
                    amount: 50.0,
                },
                PaymentSummaryItem {
                    method: "card".to_string(),
                    amount: 35.50,
                },
            ],
        };

        let hash = canonical_sha256(&payload);
        assert_eq!(
            hash, "a7474f8ed97d2a411866852e77d590cc9e850f7501721af899f952f134f4d586",
            "Golden hash mismatch — canonical encoding changed!"
        );
    }

    #[test]
    fn test_golden_payment_added() {
        let payload = EventPayload::PaymentAdded {
            payment_id: "pay-001".to_string(),
            method: "cash".to_string(),
            amount: 100.0,
            tendered: Some(120.0),
            change: Some(20.0),
            note: None,
        };

        let hash = canonical_sha256(&payload);
        assert_eq!(
            hash, "7c88ca889bc1417441aa802f39ec69c1c3fd3240313376502acb5a37b4d3a3f1",
            "Golden hash mismatch — canonical encoding changed!"
        );
    }

    #[test]
    fn test_golden_items_added() {
        let payload = EventPayload::ItemsAdded {
            items: vec![CartItemSnapshot {
                id: 1,
                instance_id: "inst-1".to_string(),
                name: "Cerveza".to_string(),
                price: 3.50,
                original_price: 3.50,
                quantity: 2,
                unpaid_quantity: 2,
                selected_options: None,
                selected_specification: None,
                manual_discount_percent: None,
                rule_discount_amount: 0.0,
                rule_surcharge_amount: 0.0,
                applied_rules: vec![],
                applied_mg_rules: vec![],
                mg_discount_amount: 0.0,
                unit_price: 3.50,
                line_total: 7.0,
                tax: 1.47,
                tax_rate: 21,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
                category_id: Some(2),
                category_name: Some("Bebidas".to_string()),
                is_comped: false,
            }],
        };

        let hash = canonical_sha256(&payload);
        assert_eq!(
            hash, "464c24c4f2d4b684b7ae3139df97b6abcaac8e79658b72b073fc5e412dd2fe1d",
            "Golden hash mismatch — canonical encoding changed!"
        );
    }

    #[test]
    fn test_golden_order_voided() {
        let payload = EventPayload::OrderVoided {
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: Some("customer cancelled".to_string()),
            authorizer_id: Some(1),
            authorizer_name: Some("Admin".to_string()),
        };

        let hash = canonical_sha256(&payload);
        assert_eq!(
            hash, "f732e83f09712b4b392a396df2894e296561909a4f44cd71dfbaa4ba6ebfe439",
            "Golden hash mismatch — canonical encoding changed!"
        );
    }

    // ========================================================================
    // D. Field order sensitivity (different variants with similar fields)
    // ========================================================================

    #[test]
    fn test_different_variants_with_authorizer_produce_different_hashes() {
        // OrderVoided, ItemRemoved, PaymentCancelled all have authorizer_id/name
        let voided = EventPayload::OrderVoided {
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: None,
            authorizer_id: Some(99),
            authorizer_name: Some("Manager".to_string()),
        };
        let removed = EventPayload::ItemRemoved {
            instance_id: "x".to_string(),
            item_name: "x".to_string(),
            quantity: None,
            reason: None,
            authorizer_id: Some(99),
            authorizer_name: Some("Manager".to_string()),
        };
        let cancelled = EventPayload::PaymentCancelled {
            payment_id: "x".to_string(),
            method: "x".to_string(),
            amount: 0.0,
            reason: None,
            authorizer_id: Some(99),
            authorizer_name: Some("Manager".to_string()),
        };

        let h_voided = canonical_sha256(&voided);
        let h_removed = canonical_sha256(&removed);
        let h_cancelled = canonical_sha256(&cancelled);

        assert_ne!(
            h_voided, h_removed,
            "OrderVoided vs ItemRemoved must differ"
        );
        assert_ne!(
            h_voided, h_cancelled,
            "OrderVoided vs PaymentCancelled must differ"
        );
        assert_ne!(
            h_removed, h_cancelled,
            "ItemRemoved vs PaymentCancelled must differ"
        );
    }

    #[test]
    fn test_moved_out_vs_merged_out_different_hash() {
        // OrderMovedOut and OrderMergedOut have the same field structure
        let moved_out = EventPayload::OrderMovedOut {
            target_table_id: 5,
            target_table_name: "Mesa 5".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };
        let merged_out = EventPayload::OrderMergedOut {
            target_table_id: 5,
            target_table_name: "Mesa 5".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        assert_ne!(
            canonical_sha256(&moved_out),
            canonical_sha256(&merged_out),
            "OrderMovedOut vs OrderMergedOut must differ even with same field values"
        );
    }

    #[test]
    fn test_item_split_vs_amount_split_different_hash() {
        let item_split = EventPayload::ItemSplit {
            payment_id: "p1".to_string(),
            split_amount: 50.0,
            payment_method: "cash".to_string(),
            items: vec![],
            tendered: None,
            change: None,
        };
        let amount_split = EventPayload::AmountSplit {
            payment_id: "p1".to_string(),
            split_amount: 50.0,
            payment_method: "cash".to_string(),
            tendered: None,
            change: None,
        };

        assert_ne!(
            canonical_sha256(&item_split),
            canonical_sha256(&amount_split),
            "ItemSplit vs AmountSplit must differ even with similar fields"
        );
    }

    // ========================================================================
    // Original tests preserved
    // ========================================================================

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

    // ========================================================================
    // E. OrderEvent hash tests
    // ========================================================================

    fn make_test_event(
        payload: EventPayload,
        event_type: OrderEventType,
    ) -> crate::order::event::OrderEvent {
        crate::order::event::OrderEvent {
            event_id: "evt-001".to_string(),
            sequence: 1,
            order_id: "ord-001".to_string(),
            timestamp: 1700000000000,
            client_timestamp: Some(1699999999000),
            operator_id: 42,
            operator_name: "Camarero".to_string(),
            command_id: "cmd-001".to_string(),
            event_type,
            payload,
        }
    }

    #[test]
    fn test_order_event_canonical_deterministic() {
        let event = make_test_event(
            EventPayload::TableOpened {
                table_id: Some(1),
                table_name: Some("Mesa 1".to_string()),
                zone_id: Some(10),
                zone_name: Some("Terraza".to_string()),
                guest_count: 4,
                is_retail: false,
                queue_number: None,
                receipt_number: "R-001".to_string(),
            },
            OrderEventType::TableOpened,
        );

        let h1 = canonical_sha256(&event);
        let h2 = canonical_sha256(&event);
        assert_eq!(h1, h2, "Same event must produce identical hashes");
    }

    #[test]
    fn test_order_event_different_metadata_different_hash() {
        let event1 = make_test_event(
            EventPayload::OrderNoteAdded {
                note: "hello".to_string(),
                previous_note: None,
            },
            OrderEventType::OrderNoteAdded,
        );
        let mut event2 = event1.clone();
        event2.operator_id = 99; // different operator

        assert_ne!(
            canonical_sha256(&event1),
            canonical_sha256(&event2),
            "Different operator_id must produce different hashes"
        );
    }

    #[test]
    fn test_order_event_json_roundtrip() {
        let event = make_test_event(
            EventPayload::PaymentAdded {
                payment_id: "pay-1".to_string(),
                method: "cash".to_string(),
                amount: 50.0,
                tendered: Some(60.0),
                change: Some(10.0),
                note: None,
            },
            OrderEventType::PaymentAdded,
        );

        let hash_before = canonical_sha256(&event);
        let json = serde_json::to_string(&event).unwrap();
        let restored: crate::order::event::OrderEvent = serde_json::from_str(&json).unwrap();
        let hash_after = canonical_sha256(&restored);
        assert_eq!(
            hash_before, hash_after,
            "OrderEvent hash must survive JSON roundtrip"
        );
    }

    #[test]
    fn test_order_event_golden_hash() {
        let event = make_test_event(
            EventPayload::TableOpened {
                table_id: Some(1),
                table_name: Some("Mesa 1".to_string()),
                zone_id: Some(10),
                zone_name: Some("Terraza".to_string()),
                guest_count: 4,
                is_retail: false,
                queue_number: None,
                receipt_number: "R-20240101-001".to_string(),
            },
            OrderEventType::TableOpened,
        );

        let hash = canonical_sha256(&event);
        assert_eq!(
            hash,
            compute_event_chain_hash(&event),
            "canonical_sha256 must match compute_event_chain_hash"
        );
        // Pin the golden value
        assert_eq!(
            hash, "9e7df918610c8f7f82993a99bebc2cf1980241a4c06b0406a32e12b8e6497b4a",
            "OrderEvent golden hash changed — canonical encoding broke!"
        );
    }

    #[test]
    fn test_compute_order_chain_hash_deterministic() {
        let h1 = compute_order_chain_hash(
            "prev",
            "ord-1",
            "R-001",
            &OrderStatus::Completed,
            "last_evt",
        );
        let h2 = compute_order_chain_hash(
            "prev",
            "ord-1",
            "R-001",
            &OrderStatus::Completed,
            "last_evt",
        );
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_compute_order_chain_hash_golden() {
        let hash = compute_order_chain_hash(
            "genesis",
            "ord-001",
            "R-20240101-001",
            &OrderStatus::Completed,
            "abc123def456",
        );
        assert_eq!(
            hash, "874db73ebf9940495ba91f7bef1fb2fed4016e26655cadbfceb58054734906fa",
            "Order chain golden hash changed!"
        );
    }

    #[test]
    fn test_compute_order_chain_hash_different_status() {
        let h_completed =
            compute_order_chain_hash("prev", "ord-1", "R-001", &OrderStatus::Completed, "last");
        let h_voided =
            compute_order_chain_hash("prev", "ord-1", "R-001", &OrderStatus::Void, "last");
        assert_ne!(
            h_completed, h_voided,
            "Different status must produce different hash"
        );
    }

    #[test]
    fn test_order_event_client_timestamp_none_vs_some() {
        let mut event1 = make_test_event(
            EventPayload::OrderNoteAdded {
                note: "test".to_string(),
                previous_note: None,
            },
            OrderEventType::OrderNoteAdded,
        );
        let mut event2 = event1.clone();
        event1.client_timestamp = None;
        event2.client_timestamp = Some(0);

        assert_ne!(
            canonical_sha256(&event1),
            canonical_sha256(&event2),
            "client_timestamp None vs Some(0) must differ"
        );
    }
}
