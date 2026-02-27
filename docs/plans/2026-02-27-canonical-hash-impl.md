# CanonicalHash Trait Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 hash chain 计算从依赖 serde_json 改为使用确定性二进制协议，确保序列化/反序列化 roundtrip 后 hash 不变。

**Architecture:** 在 `shared` crate 新增 `CanonicalHash` trait，所有参与 hash 计算的类型实现该 trait。hash 计算函数从 `serde_json::to_string` 改为 `canonical_bytes`。编译器通过 exhaustive match 强制新增 variant 时必须实现。

**Tech Stack:** Rust, SHA-256 (`sha2` crate), `shared` crate (类型定义), `edge-server` crate (hash 计算)

**Design Doc:** `docs/plans/2026-02-27-canonical-hash-design.md`

---

### Task 1: 创建 CanonicalHash trait 和辅助函数

**Files:**
- Create: `shared/src/order/canonical.rs`
- Modify: `shared/src/order/mod.rs`

**Step 1: 创建 `canonical.rs` 文件**

```rust
//! Canonical binary representation for hash chain integrity.
//!
//! All types participating in hash chain computation implement `CanonicalHash`,
//! producing identical bytes for identical data regardless of serde configuration,
//! field ordering, or Rust compiler version.

/// Trait for deterministic binary serialization used in hash chain computation.
///
/// Implementors MUST guarantee:
/// 1. Same data always produces same bytes (determinism)
/// 2. Different data always produces different bytes (collision resistance)
/// 3. Output is independent of serde attributes and field declaration order
pub trait CanonicalHash {
    fn canonical_bytes(&self, buf: &mut Vec<u8>);
}

// ============================================================================
// Helper functions for writing primitive types
// ============================================================================

const SEP: u8 = 0x00;
const NONE_TAG: u8 = 0x00;
const SOME_TAG: u8 = 0x01;

#[inline]
pub(crate) fn write_sep(buf: &mut Vec<u8>) {
    buf.push(SEP);
}

#[inline]
pub(crate) fn write_i64(buf: &mut Vec<u8>, v: i64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
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
    buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
    buf.extend_from_slice(s.as_bytes());
}

#[inline]
pub(crate) fn write_tag(buf: &mut Vec<u8>, tag: &[u8]) {
    buf.extend_from_slice(tag);
}

pub(crate) fn write_opt<T: CanonicalHash>(buf: &mut Vec<u8>, opt: &Option<T>) {
    match opt {
        None => buf.push(NONE_TAG),
        Some(v) => {
            buf.push(SOME_TAG);
            v.canonical_bytes(buf);
        }
    }
}

pub(crate) fn write_opt_i64(buf: &mut Vec<u8>, opt: Option<i64>) {
    match opt {
        None => buf.push(NONE_TAG),
        Some(v) => {
            buf.push(SOME_TAG);
            write_i64(buf, v);
        }
    }
}

pub(crate) fn write_opt_i32(buf: &mut Vec<u8>, opt: Option<i32>) {
    match opt {
        None => buf.push(NONE_TAG),
        Some(v) => {
            buf.push(SOME_TAG);
            write_i32(buf, v);
        }
    }
}

pub(crate) fn write_opt_u32(buf: &mut Vec<u8>, opt: Option<u32>) {
    match opt {
        None => buf.push(NONE_TAG),
        Some(v) => {
            buf.push(SOME_TAG);
            write_u32(buf, v);
        }
    }
}

pub(crate) fn write_opt_f64(buf: &mut Vec<u8>, opt: Option<f64>) {
    match opt {
        None => buf.push(NONE_TAG),
        Some(v) => {
            buf.push(SOME_TAG);
            write_f64(buf, v);
        }
    }
}

pub(crate) fn write_opt_str(buf: &mut Vec<u8>, opt: &Option<String>) {
    match opt {
        None => buf.push(NONE_TAG),
        Some(s) => {
            buf.push(SOME_TAG);
            write_str(buf, s);
        }
    }
}

pub(crate) fn write_opt_bool(buf: &mut Vec<u8>, opt: Option<bool>) {
    match opt {
        None => buf.push(NONE_TAG),
        Some(v) => {
            buf.push(SOME_TAG);
            write_bool(buf, v);
        }
    }
}

pub(crate) fn write_vec<T: CanonicalHash>(buf: &mut Vec<u8>, items: &[T]) {
    write_u32(buf, items.len() as u32);
    for item in items {
        item.canonical_bytes(buf);
    }
}

pub(crate) fn write_btreemap_str_i32(
    buf: &mut Vec<u8>,
    map: &std::collections::BTreeMap<String, i32>,
) {
    write_u32(buf, map.len() as u32);
    for (k, v) in map {
        write_str(buf, k);
        write_i32(buf, *v);
    }
}

// ============================================================================
// Primitive wrapper impls (for Option<T> where T is a String, etc.)
// ============================================================================

impl CanonicalHash for String {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_str(buf, self);
    }
}
```

**Step 2: 注册模块**

在 `shared/src/order/mod.rs` 添加:
```rust
pub mod canonical;
pub use canonical::CanonicalHash;
```

**Step 3: 编译检查**

Run: `cargo check -p shared`
Expected: PASS

**Step 4: Commit**

```bash
git add shared/src/order/canonical.rs shared/src/order/mod.rs
git commit -m "feat(shared): add CanonicalHash trait and helper functions"
```

---

### Task 2: 实现枚举类型的 CanonicalHash

**Files:**
- Modify: `shared/src/order/canonical.rs`

为所有小型枚举实现 CanonicalHash。每个枚举用固定 ASCII tag 标识 variant。

**Step 1: 添加枚举实现**

在 `canonical.rs` 末尾添加:

```rust
use super::snapshot::OrderStatus;
use super::event::OrderEventType;
use super::types::{VoidType, LossReason, ServiceType, SplitType};
use crate::models::price_rule::{RuleType, AdjustmentType, ProductScope};

impl CanonicalHash for OrderStatus {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Active => write_tag(buf, b"ACTIVE"),
            Self::Completed => write_tag(buf, b"COMPLETED"),
            Self::Void => write_tag(buf, b"VOID"),
            Self::Merged => write_tag(buf, b"MERGED"),
        }
    }
}

impl CanonicalHash for OrderEventType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::TableOpened => write_tag(buf, b"TABLE_OPENED"),
            Self::OrderCompleted => write_tag(buf, b"ORDER_COMPLETED"),
            Self::OrderVoided => write_tag(buf, b"ORDER_VOIDED"),
            Self::ItemsAdded => write_tag(buf, b"ITEMS_ADDED"),
            Self::ItemModified => write_tag(buf, b"ITEM_MODIFIED"),
            Self::ItemRemoved => write_tag(buf, b"ITEM_REMOVED"),
            Self::ItemComped => write_tag(buf, b"ITEM_COMPED"),
            Self::ItemUncomped => write_tag(buf, b"ITEM_UNCOMPED"),
            Self::PaymentAdded => write_tag(buf, b"PAYMENT_ADDED"),
            Self::PaymentCancelled => write_tag(buf, b"PAYMENT_CANCELLED"),
            Self::ItemSplit => write_tag(buf, b"ITEM_SPLIT"),
            Self::AmountSplit => write_tag(buf, b"AMOUNT_SPLIT"),
            Self::AaSplitStarted => write_tag(buf, b"AA_SPLIT_STARTED"),
            Self::AaSplitPaid => write_tag(buf, b"AA_SPLIT_PAID"),
            Self::AaSplitCancelled => write_tag(buf, b"AA_SPLIT_CANCELLED"),
            Self::OrderMoved => write_tag(buf, b"ORDER_MOVED"),
            Self::OrderMovedOut => write_tag(buf, b"ORDER_MOVED_OUT"),
            Self::OrderMerged => write_tag(buf, b"ORDER_MERGED"),
            Self::OrderMergedOut => write_tag(buf, b"ORDER_MERGED_OUT"),
            Self::TableReassigned => write_tag(buf, b"TABLE_REASSIGNED"),
            Self::OrderInfoUpdated => write_tag(buf, b"ORDER_INFO_UPDATED"),
            Self::RuleSkipToggled => write_tag(buf, b"RULE_SKIP_TOGGLED"),
            Self::OrderDiscountApplied => write_tag(buf, b"ORDER_DISCOUNT_APPLIED"),
            Self::OrderSurchargeApplied => write_tag(buf, b"ORDER_SURCHARGE_APPLIED"),
            Self::OrderNoteAdded => write_tag(buf, b"ORDER_NOTE_ADDED"),
            Self::MemberLinked => write_tag(buf, b"MEMBER_LINKED"),
            Self::MemberUnlinked => write_tag(buf, b"MEMBER_UNLINKED"),
            Self::StampRedeemed => write_tag(buf, b"STAMP_REDEEMED"),
            Self::StampRedemptionCancelled => write_tag(buf, b"STAMP_REDEMPTION_CANCELLED"),
        }
    }
}

impl CanonicalHash for VoidType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Cancelled => write_tag(buf, b"CANCELLED"),
            Self::LossSettled => write_tag(buf, b"LOSS_SETTLED"),
        }
    }
}

impl CanonicalHash for LossReason {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::CustomerFled => write_tag(buf, b"CUSTOMER_FLED"),
            Self::RefusedToPay => write_tag(buf, b"REFUSED_TO_PAY"),
            Self::Other => write_tag(buf, b"OTHER"),
        }
    }
}

impl CanonicalHash for ServiceType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::DineIn => write_tag(buf, b"DINE_IN"),
            Self::Takeout => write_tag(buf, b"TAKEOUT"),
        }
    }
}

impl CanonicalHash for SplitType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::ItemSplit => write_tag(buf, b"ITEM_SPLIT"),
            Self::AmountSplit => write_tag(buf, b"AMOUNT_SPLIT"),
            Self::AaSplit => write_tag(buf, b"AA_SPLIT"),
        }
    }
}

impl CanonicalHash for RuleType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Discount => write_tag(buf, b"DISCOUNT"),
            Self::Surcharge => write_tag(buf, b"SURCHARGE"),
        }
    }
}

impl CanonicalHash for AdjustmentType {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Percentage => write_tag(buf, b"PERCENTAGE"),
            Self::FixedAmount => write_tag(buf, b"FIXED_AMOUNT"),
        }
    }
}

impl CanonicalHash for ProductScope {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Global => write_tag(buf, b"GLOBAL"),
            Self::Category => write_tag(buf, b"CATEGORY"),
            Self::Tag => write_tag(buf, b"TAG"),
            Self::Product => write_tag(buf, b"PRODUCT"),
        }
    }
}
```

**Step 2: 编译检查**

Run: `cargo check -p shared`
Expected: PASS

**Step 3: Commit**

```bash
git add shared/src/order/canonical.rs
git commit -m "feat(shared): implement CanonicalHash for enum types"
```

---

### Task 3: 实现 struct 类型的 CanonicalHash (小型)

**Files:**
- Modify: `shared/src/order/canonical.rs`

为 `ItemOption`, `SpecificationInfo`, `SplitItem`, `ItemChanges`, `ItemModificationResult`, `PaymentSummaryItem`, `MgItemDiscount` 实现 CanonicalHash。

**Step 1: 添加 struct 实现**

```rust
use super::types::{
    CartItemSnapshot, ItemChanges, ItemModificationResult, ItemOption, PaymentRecord,
    SplitItem, SpecificationInfo, CompRecord, StampRedemptionState, PaymentSummaryItem,
};
use super::event::MgItemDiscount;
use super::{AppliedRule, AppliedMgRule};

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
        match &self.selected_options {
            None => buf.push(NONE_TAG),
            Some(opts) => {
                buf.push(SOME_TAG);
                write_vec(buf, opts);
            }
        }
        match &self.selected_specification {
            None => buf.push(NONE_TAG),
            Some(spec) => {
                buf.push(SOME_TAG);
                spec.canonical_bytes(buf);
            }
        }
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
```

**Step 2: 编译检查**

Run: `cargo check -p shared`
Expected: PASS

**Step 3: Commit**

```bash
git add shared/src/order/canonical.rs
git commit -m "feat(shared): implement CanonicalHash for small struct types"
```

---

### Task 4: 实现 CartItemSnapshot 和 PaymentRecord 的 CanonicalHash

**Files:**
- Modify: `shared/src/order/canonical.rs`

这两个是最大的 struct，字段最多。

**Step 1: 添加实现**

```rust
impl CanonicalHash for CartItemSnapshot {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        write_i64(buf, self.id);
        write_str(buf, &self.instance_id);
        write_str(buf, &self.name);
        write_f64(buf, self.price);
        write_f64(buf, self.original_price);
        write_i32(buf, self.quantity);
        write_i32(buf, self.unpaid_quantity);
        match &self.selected_options {
            None => buf.push(NONE_TAG),
            Some(opts) => {
                buf.push(SOME_TAG);
                write_vec(buf, opts);
            }
        }
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
        match &self.split_items {
            None => buf.push(NONE_TAG),
            Some(items) => {
                buf.push(SOME_TAG);
                write_vec(buf, items);
            }
        }
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
```

**Step 2: 编译检查**

Run: `cargo check -p shared`
Expected: PASS

**Step 3: Commit**

```bash
git add shared/src/order/canonical.rs
git commit -m "feat(shared): implement CanonicalHash for CartItemSnapshot and PaymentRecord"
```

---

### Task 5: 实现 EventPayload 的 CanonicalHash

**Files:**
- Modify: `shared/src/order/canonical.rs`

这是最大的实现——26 个 variant，每个手动指定字段顺序。

**Step 1: 添加 EventPayload 实现**

```rust
use super::event::EventPayload;

impl CanonicalHash for EventPayload {
    fn canonical_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            // ========== Lifecycle ==========
            Self::TableOpened {
                table_id, table_name, zone_id, zone_name,
                guest_count, is_retail, queue_number, receipt_number,
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

            Self::OrderCompleted {
                receipt_number, service_type, final_total, payment_summary,
            } => {
                write_tag(buf, b"ORDER_COMPLETED");
                write_sep(buf);
                write_str(buf, receipt_number);
                write_opt(buf, service_type);
                write_f64(buf, *final_total);
                write_vec(buf, payment_summary);
            }

            Self::OrderVoided {
                void_type, loss_reason, loss_amount, note,
                authorizer_id, authorizer_name,
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

            // ========== Items ==========
            Self::ItemsAdded { items } => {
                write_tag(buf, b"ITEMS_ADDED");
                write_sep(buf);
                write_vec(buf, items);
            }

            Self::ItemModified {
                operation, source, affected_quantity, changes,
                previous_values, results, authorizer_id, authorizer_name,
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

            Self::ItemRemoved {
                instance_id, item_name, quantity, reason,
                authorizer_id, authorizer_name,
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

            Self::ItemComped {
                instance_id, source_instance_id, item_name,
                quantity, original_price, reason,
                authorizer_id, authorizer_name,
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

            Self::ItemUncomped {
                instance_id, item_name, restored_price,
                merged_into, authorizer_id, authorizer_name,
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

            // ========== Payments ==========
            Self::PaymentAdded {
                payment_id, method, amount, tendered, change, note,
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

            Self::PaymentCancelled {
                payment_id, method, amount, reason,
                authorizer_id, authorizer_name,
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

            // ========== Split ==========
            Self::ItemSplit {
                payment_id, split_amount, payment_method,
                items, tendered, change,
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

            Self::AmountSplit {
                payment_id, split_amount, payment_method, tendered, change,
            } => {
                write_tag(buf, b"AMOUNT_SPLIT");
                write_sep(buf);
                write_str(buf, payment_id);
                write_f64(buf, *split_amount);
                write_str(buf, payment_method);
                write_opt_f64(buf, *tendered);
                write_opt_f64(buf, *change);
            }

            Self::AaSplitStarted {
                total_shares, per_share_amount, order_total,
            } => {
                write_tag(buf, b"AA_SPLIT_STARTED");
                write_sep(buf);
                write_i32(buf, *total_shares);
                write_f64(buf, *per_share_amount);
                write_f64(buf, *order_total);
            }

            Self::AaSplitPaid {
                payment_id, shares, amount, payment_method,
                progress_paid, progress_total, tendered, change,
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

            Self::AaSplitCancelled { total_shares } => {
                write_tag(buf, b"AA_SPLIT_CANCELLED");
                write_sep(buf);
                write_i32(buf, *total_shares);
            }

            // ========== Table Operations ==========
            Self::OrderMoved {
                source_table_id, source_table_name,
                target_table_id, target_table_name,
                target_zone_id, target_zone_name,
                items, authorizer_id, authorizer_name,
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

            Self::OrderMovedOut {
                target_table_id, target_table_name,
                reason, authorizer_id, authorizer_name,
            } => {
                write_tag(buf, b"ORDER_MOVED_OUT");
                write_sep(buf);
                write_i64(buf, *target_table_id);
                write_str(buf, target_table_name);
                write_opt_str(buf, reason);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            Self::OrderMerged {
                source_table_id, source_table_name,
                items, payments, paid_item_quantities,
                paid_amount, has_amount_split,
                aa_total_shares, aa_paid_shares,
                authorizer_id, authorizer_name,
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

            Self::OrderMergedOut {
                target_table_id, target_table_name,
                reason, authorizer_id, authorizer_name,
            } => {
                write_tag(buf, b"ORDER_MERGED_OUT");
                write_sep(buf);
                write_i64(buf, *target_table_id);
                write_str(buf, target_table_name);
                write_opt_str(buf, reason);
                write_opt_i64(buf, *authorizer_id);
                write_opt_str(buf, authorizer_name);
            }

            Self::TableReassigned {
                source_table_id, source_table_name,
                target_table_id, target_table_name,
                target_zone_name, original_start_time, items,
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

            // ========== Other ==========
            Self::OrderInfoUpdated {
                guest_count, table_name, is_pre_payment,
            } => {
                write_tag(buf, b"ORDER_INFO_UPDATED");
                write_sep(buf);
                write_opt_i32(buf, *guest_count);
                write_opt_str(buf, table_name);
                write_opt_bool(buf, *is_pre_payment);
            }

            Self::RuleSkipToggled {
                rule_id, rule_name, skipped,
            } => {
                write_tag(buf, b"RULE_SKIP_TOGGLED");
                write_sep(buf);
                write_i64(buf, *rule_id);
                write_str(buf, rule_name);
                write_bool(buf, *skipped);
            }

            Self::OrderDiscountApplied {
                discount_percent, discount_fixed,
                previous_discount_percent, previous_discount_fixed,
                authorizer_id, authorizer_name,
                subtotal, discount, total,
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

            Self::OrderSurchargeApplied {
                surcharge_percent, surcharge_amount,
                previous_surcharge_percent, previous_surcharge_amount,
                authorizer_id, authorizer_name,
                subtotal, surcharge, total,
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

            Self::OrderNoteAdded { note, previous_note } => {
                write_tag(buf, b"ORDER_NOTE_ADDED");
                write_sep(buf);
                write_str(buf, note);
                write_opt_str(buf, previous_note);
            }

            // ========== Member ==========
            Self::MemberLinked {
                member_id, member_name,
                marketing_group_id, marketing_group_name,
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

            Self::MemberUnlinked {
                previous_member_id, previous_member_name,
            } => {
                write_tag(buf, b"MEMBER_UNLINKED");
                write_sep(buf);
                write_i64(buf, *previous_member_id);
                write_str(buf, previous_member_name);
            }

            Self::StampRedeemed {
                stamp_activity_id, stamp_activity_name,
                reward_instance_id, reward_strategy,
                product_id, product_name,
                original_price, quantity, tax_rate,
                category_id, category_name,
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

            Self::StampRedemptionCancelled {
                stamp_activity_id, stamp_activity_name,
                reward_instance_id, is_comp_existing,
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
```

**Step 2: 编译检查**

Run: `cargo check -p shared`
Expected: PASS

**Step 3: Commit**

```bash
git add shared/src/order/canonical.rs
git commit -m "feat(shared): implement CanonicalHash for EventPayload (26 variants)"
```

---

### Task 6: 添加 Golden Tests

**Files:**
- Modify: `shared/src/order/canonical.rs` (追加 `#[cfg(test)]` 模块)

Golden test 固定输入 → 固定 hash 输出，防止回归。

**Step 1: 添加测试**

在 `canonical.rs` 末尾添加:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Sha256, Digest};
    use super::super::event::{EventPayload, OrderEventType};
    use super::super::snapshot::OrderStatus;
    use super::super::types::*;

    fn canonical_sha256(payload: &impl CanonicalHash) -> String {
        let mut buf = Vec::new();
        payload.canonical_bytes(&mut buf);
        format!("{:x}", Sha256::digest(&buf))
    }

    // ── Determinism tests ──

    #[test]
    fn test_canonical_deterministic() {
        let payload = EventPayload::TableOpened {
            table_id: Some(1),
            table_name: Some("Mesa 1".to_string()),
            zone_id: Some(10),
            zone_name: Some("Sala".to_string()),
            guest_count: 4,
            is_retail: false,
            queue_number: None,
            receipt_number: "RCP-001".to_string(),
        };

        let hash1 = canonical_sha256(&payload);
        let hash2 = canonical_sha256(&payload);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_canonical_different_values_different_hash() {
        let p1 = EventPayload::TableOpened {
            table_id: Some(1),
            table_name: Some("Mesa 1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            queue_number: None,
            receipt_number: "RCP-001".to_string(),
        };

        let p2 = EventPayload::TableOpened {
            table_id: Some(2),
            table_name: Some("Mesa 2".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            queue_number: None,
            receipt_number: "RCP-001".to_string(),
        };

        assert_ne!(canonical_sha256(&p1), canonical_sha256(&p2));
    }

    #[test]
    fn test_canonical_none_vs_some_different() {
        let p1 = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: 10.0,
            tendered: None,
            change: None,
            note: None,
        };

        let p2 = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: 10.0,
            tendered: Some(20.0),
            change: Some(10.0),
            note: None,
        };

        assert_ne!(canonical_sha256(&p1), canonical_sha256(&p2));
    }

    #[test]
    fn test_canonical_roundtrip_stable() {
        // Serialize → Deserialize → canonical_bytes should be identical
        let payload = EventPayload::ItemsAdded {
            items: vec![CartItemSnapshot {
                id: 1,
                instance_id: "inst-1".to_string(),
                name: "Cafe".to_string(),
                price: 2.5,
                original_price: 3.0,
                quantity: 2,
                unpaid_quantity: 2,
                selected_options: Some(vec![ItemOption {
                    attribute_id: 1,
                    attribute_name: "Size".to_string(),
                    option_id: 2,
                    option_name: "Large".to_string(),
                    price_modifier: Some(0.5),
                    quantity: 1,
                    receipt_name: None,
                    kitchen_print_name: None,
                    show_on_receipt: true,
                    show_on_kitchen_print: true,
                }]),
                selected_specification: None,
                manual_discount_percent: Some(10.0),
                rule_discount_amount: 0.0,
                rule_surcharge_amount: 0.0,
                applied_rules: vec![],
                applied_mg_rules: vec![],
                mg_discount_amount: 0.0,
                unit_price: 2.5,
                line_total: 5.0,
                tax: 1.05,
                tax_rate: 21,
                note: Some("sin leche".to_string()),
                authorizer_id: None,
                authorizer_name: None,
                category_id: Some(5),
                category_name: Some("Bebidas".to_string()),
                is_comped: false,
            }],
        };

        let hash_before = canonical_sha256(&payload);

        // Serialize to JSON
        let json = serde_json::to_string(&payload).unwrap();
        // Deserialize back
        let restored: EventPayload = serde_json::from_str(&json).unwrap();

        let hash_after = canonical_sha256(&restored);
        assert_eq!(hash_before, hash_after, "canonical hash must survive serde roundtrip");
    }

    #[test]
    fn test_canonical_f64_roundtrip_stable() {
        let payload = EventPayload::PaymentAdded {
            payment_id: "p1".to_string(),
            method: "cash".to_string(),
            amount: 12.34,
            tendered: Some(20.0),
            change: Some(7.66),
            note: None,
        };

        let hash_before = canonical_sha256(&payload);
        let json = serde_json::to_string(&payload).unwrap();
        let restored: EventPayload = serde_json::from_str(&json).unwrap();
        let hash_after = canonical_sha256(&restored);
        assert_eq!(hash_before, hash_after);
    }

    #[test]
    fn test_canonical_all_event_types_covered() {
        // This test ensures every OrderEventType variant produces unique bytes
        let all_types = vec![
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
            assert!(hashes.insert(h), "Duplicate canonical hash for {:?}", et);
        }
    }

    #[test]
    fn test_canonical_order_status_all_unique() {
        let statuses = vec![
            OrderStatus::Active,
            OrderStatus::Completed,
            OrderStatus::Void,
            OrderStatus::Merged,
        ];
        let mut hashes = std::collections::HashSet::new();
        for s in &statuses {
            assert!(hashes.insert(canonical_sha256(s)));
        }
    }

    // ── Golden tests (fixed input → fixed output) ──

    #[test]
    fn test_golden_table_opened() {
        let payload = EventPayload::TableOpened {
            table_id: Some(1),
            table_name: Some("Mesa 1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            queue_number: None,
            receipt_number: "RCP-20260227-0001".to_string(),
        };

        let hash = canonical_sha256(&payload);
        // Record the golden value on first run, then hardcode
        // This ensures future code changes don't accidentally alter canonical format
        assert_eq!(hash.len(), 64);
        // Snapshot: uncomment after first run
        // assert_eq!(hash, "<paste hash here>");
    }
}
```

**Step 2: 运行测试**

Run: `cargo test -p shared --lib order::canonical`
Expected: 全部 PASS

**Step 3: 固化 golden 值**

运行测试，获取 `test_golden_table_opened` 的实际 hash 值，取消注释并填入 `assert_eq!`。

**Step 4: Commit**

```bash
git add shared/src/order/canonical.rs
git commit -m "test(shared): add canonical hash determinism and golden tests"
```

---

### Task 7: 改造 edge-server hash 计算函数

**Files:**
- Modify: `edge-server/src/archiving/service.rs`
  - `compute_event_hash()` 方法 (~L698-715)
  - `compute_order_hash()` 方法 (~L675-695)
  - 测试中的 `compute_event_hash_standalone()` (~L1038-1051)
  - 测试中的 `compute_order_hash_standalone()` (~L1019-1036)

**Step 1: 修改 `compute_event_hash`**

替换现有实现:

```rust
fn compute_event_hash(&self, event: &OrderEvent) -> String {
    use shared::order::CanonicalHash;

    let mut buf = Vec::with_capacity(512);
    buf.extend_from_slice(event.event_id.as_bytes());
    buf.push(0x00);
    buf.extend_from_slice(event.order_id.as_bytes());
    buf.push(0x00);
    buf.extend_from_slice(&event.sequence.to_le_bytes());
    event.event_type.canonical_bytes(&mut buf);
    buf.push(0x00);
    event.payload.canonical_bytes(&mut buf);

    format!("{:x}", Sha256::digest(&buf))
}
```

**Step 2: 修改 `compute_order_hash`**

```rust
fn compute_order_hash(
    &self,
    snapshot: &OrderSnapshot,
    prev_hash: &str,
    last_event_hash: &str,
) -> String {
    use shared::order::CanonicalHash;

    let mut buf = Vec::with_capacity(256);
    buf.extend_from_slice(prev_hash.as_bytes());
    buf.push(0x00);
    buf.extend_from_slice(snapshot.order_id.as_bytes());
    buf.push(0x00);
    buf.extend_from_slice(snapshot.receipt_number.as_bytes());
    buf.push(0x00);
    snapshot.status.canonical_bytes(&mut buf);
    buf.push(0x00);
    buf.extend_from_slice(last_event_hash.as_bytes());

    format!("{:x}", Sha256::digest(&buf))
}
```

**Step 3: 同步更新测试中的 standalone 函数**

将 `compute_event_hash_standalone` 和 `compute_order_hash_standalone` 改为使用相同的 canonical 逻辑（复制上面的代码，去掉 `&self`）。

**Step 4: 编译检查**

Run: `cargo check -p edge-server`
Expected: PASS

**Step 5: 运行测试**

Run: `cargo test -p edge-server --lib archiving`
Expected: 全部 PASS（hash 值会变，但确定性和链完整性测试应该通过）

**Step 6: Commit**

```bash
git add edge-server/src/archiving/service.rs
git commit -m "feat(archiving): switch hash computation to CanonicalHash trait"
```

---

### Task 8: 最终验证

**Step 1: 全量编译检查**

Run: `cargo check --workspace`
Expected: PASS

**Step 2: 全量测试**

Run: `cargo test --workspace --lib`
Expected: 全部 PASS

**Step 3: Clippy 检查**

Run: `cargo clippy --workspace`
Expected: 无 warning

**Step 4: 固化所有 golden test 值**

确保 `test_golden_table_opened` 中的 assert_eq 已经填入了实际 hash 值。

**Step 5: Final Commit (如有残留)**

```bash
git add -A
git commit -m "chore: finalize canonical hash implementation"
```
