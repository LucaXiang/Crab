# Price Rule Order Integration - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate PriceRule system into order processing, enabling automatic price adjustments when adding/modifying items.

**Architecture:** Rules are loaded into memory on OpenTable, cached per order. Item prices are calculated using a multi-step formula: manual discount → rule discount (with stackable/exclusive logic) → rule surcharge. All percentage discounts use multiplication for "capitalist mode".

**Tech Stack:** Rust, SurrealDB, Event Sourcing (redb), rust_decimal for precision

---

## Phase 1: Update Data Structures

### Task 1.1: Update PriceRule in shared

**Files:**
- Modify: `shared/src/models/price_rule.rs`

**Step 1: Write the test for new fields**

```rust
// Add to shared/src/models/price_rule.rs at the end

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_rule_time_fields() {
        let rule = PriceRule {
            id: Some("rule-1".to_string()),
            name: "test".to_string(),
            display_name: "Test Rule".to_string(),
            receipt_name: "TEST".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            valid_from: Some(1704067200000),  // 2024-01-01
            valid_until: Some(1735689600000), // 2025-01-01
            active_days: Some(vec![1, 2, 3, 4, 5]), // Mon-Fri
            active_start_time: Some("11:00".to_string()),
            active_end_time: Some("14:00".to_string()),
            is_active: true,
            created_by: None,
            created_at: 1704067200000,
        };

        assert!(rule.is_exclusive == false);
        assert!(rule.valid_from.is_some());
        assert_eq!(rule.active_days.as_ref().unwrap().len(), 5);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p shared price_rule::tests::test_price_rule_time_fields
```
Expected: FAIL - missing fields

**Step 3: Update PriceRule struct**

Replace the entire `shared/src/models/price_rule.rs` with:

```rust
//! Price Rule Model

use serde::{Deserialize, Serialize};

/// Rule type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleType {
    Discount,
    Surcharge,
}

/// Product scope enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProductScope {
    Global,
    Category,
    Tag,
    Product,
}

/// Adjustment type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdjustmentType {
    Percentage,
    FixedAmount,
}

/// Price rule entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRule {
    pub id: Option<String>,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,

    // === Rule Type ===
    pub rule_type: RuleType,
    pub adjustment_type: AdjustmentType,
    /// Value (percentage: 10=10%, fixed: cents)
    pub adjustment_value: i32,

    // === Scope ===
    pub product_scope: ProductScope,
    /// Target ID based on scope (category/tag/product ID)
    pub target: Option<String>,
    /// Zone scope: -1=all, 0=retail, >0=specific zone
    pub zone_scope: i32,

    // === Priority & Stacking ===
    pub priority: i32,
    pub is_stackable: bool,
    /// Exclusive rule - if matched, no other rules apply
    #[serde(default)]
    pub is_exclusive: bool,

    // === Time Control ===
    /// Valid from timestamp (None = immediately effective)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<i64>,
    /// Valid until timestamp (None = never expires)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<i64>,
    /// Active days of week (0=Sunday, None = every day)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_days: Option<Vec<u8>>,
    /// Daily start time "HH:MM"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_start_time: Option<String>,
    /// Daily end time "HH:MM"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_end_time: Option<String>,

    // === Status ===
    pub is_active: bool,
    pub created_by: Option<String>,
    /// Created timestamp (for same-priority ordering)
    #[serde(default)]
    pub created_at: i64,
}

/// Create price rule payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRuleCreate {
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,
    pub rule_type: RuleType,
    pub product_scope: ProductScope,
    pub target: Option<String>,
    pub zone_scope: Option<i32>,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: i32,
    pub priority: Option<i32>,
    pub is_stackable: Option<bool>,
    pub is_exclusive: Option<bool>,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub active_days: Option<Vec<u8>>,
    pub active_start_time: Option<String>,
    pub active_end_time: Option<String>,
    pub created_by: Option<String>,
}

/// Update price rule payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRuleUpdate {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub receipt_name: Option<String>,
    pub description: Option<String>,
    pub rule_type: Option<RuleType>,
    pub product_scope: Option<ProductScope>,
    pub target: Option<String>,
    pub zone_scope: Option<i32>,
    pub adjustment_type: Option<AdjustmentType>,
    pub adjustment_value: Option<i32>,
    pub priority: Option<i32>,
    pub is_stackable: Option<bool>,
    pub is_exclusive: Option<bool>,
    pub valid_from: Option<i64>,
    pub valid_until: Option<i64>,
    pub active_days: Option<Vec<u8>>,
    pub active_start_time: Option<String>,
    pub active_end_time: Option<String>,
    pub is_active: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_rule_time_fields() {
        let rule = PriceRule {
            id: Some("rule-1".to_string()),
            name: "test".to_string(),
            display_name: "Test Rule".to_string(),
            receipt_name: "TEST".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            valid_from: Some(1704067200000),
            valid_until: Some(1735689600000),
            active_days: Some(vec![1, 2, 3, 4, 5]),
            active_start_time: Some("11:00".to_string()),
            active_end_time: Some("14:00".to_string()),
            is_active: true,
            created_by: None,
            created_at: 1704067200000,
        };

        assert!(!rule.is_exclusive);
        assert!(rule.valid_from.is_some());
        assert_eq!(rule.active_days.as_ref().unwrap().len(), 5);
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p shared price_rule::tests::test_price_rule_time_fields
```
Expected: PASS

**Step 5: Commit**

```bash
git add shared/src/models/price_rule.rs
git commit -m "feat(shared): update PriceRule with time control and exclusive fields"
```

---

### Task 1.2: Add AppliedRule struct

**Files:**
- Create: `shared/src/order/applied_rule.rs`
- Modify: `shared/src/order/mod.rs`

**Step 1: Create the AppliedRule module**

Create `shared/src/order/applied_rule.rs`:

```rust
//! Applied Rule - tracks which rules were applied to an item/order

use crate::models::price_rule::{AdjustmentType, ProductScope, RuleType};
use serde::{Deserialize, Serialize};

/// Applied rule record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppliedRule {
    // === Rule Identity ===
    pub rule_id: String,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,

    // === Rule Type ===
    pub rule_type: RuleType,
    pub adjustment_type: AdjustmentType,

    // === Scope Info ===
    pub product_scope: ProductScope,
    pub zone_scope: i32,

    // === Calculation Info ===
    /// Original value (10 = 10% or ¥10)
    pub adjustment_value: f64,
    /// Calculated amount after applying rule
    pub calculated_amount: f64,
    pub priority: i32,
    pub is_stackable: bool,
    pub is_exclusive: bool,

    // === Control ===
    /// Whether this rule is skipped
    #[serde(default)]
    pub skipped: bool,
}

impl AppliedRule {
    /// Create from a PriceRule with calculated amount
    pub fn from_rule(
        rule: &crate::models::price_rule::PriceRule,
        calculated_amount: f64,
    ) -> Self {
        Self {
            rule_id: rule.id.clone().unwrap_or_default(),
            name: rule.name.clone(),
            display_name: rule.display_name.clone(),
            receipt_name: rule.receipt_name.clone(),
            rule_type: rule.rule_type.clone(),
            adjustment_type: rule.adjustment_type.clone(),
            product_scope: rule.product_scope.clone(),
            zone_scope: rule.zone_scope,
            adjustment_value: rule.adjustment_value as f64,
            calculated_amount,
            priority: rule.priority,
            is_stackable: rule.is_stackable,
            is_exclusive: rule.is_exclusive,
            skipped: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::price_rule::PriceRule;

    #[test]
    fn test_applied_rule_from_rule() {
        let rule = PriceRule {
            id: Some("rule-1".to_string()),
            name: "lunch".to_string(),
            display_name: "Lunch Discount".to_string(),
            receipt_name: "LUNCH".to_string(),
            description: None,
            rule_type: RuleType::Discount,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: AdjustmentType::Percentage,
            adjustment_value: 10,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 0,
        };

        let applied = AppliedRule::from_rule(&rule, 5.0);

        assert_eq!(applied.rule_id, "rule-1");
        assert_eq!(applied.calculated_amount, 5.0);
        assert!(!applied.skipped);
    }
}
```

**Step 2: Update mod.rs to export AppliedRule**

Add to `shared/src/order/mod.rs`:

```rust
mod applied_rule;
pub use applied_rule::AppliedRule;
```

**Step 3: Run test**

```bash
cargo test -p shared order::applied_rule::tests
```
Expected: PASS

**Step 4: Commit**

```bash
git add shared/src/order/applied_rule.rs shared/src/order/mod.rs
git commit -m "feat(shared): add AppliedRule struct for tracking applied price rules"
```

---

### Task 1.3: Update CartItemSnapshot with rule fields

**Files:**
- Modify: `shared/src/order/types.rs`

**Step 1: Add test for new fields**

Add to `shared/src/order/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cart_item_snapshot_rule_fields() {
        let item = CartItemSnapshot {
            id: "prod-1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Test".to_string(),
            price: 100.0,
            original_price: Some(120.0),
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: Some(10.0),
            rule_discount_amount: Some(5.0),
            rule_surcharge_amount: Some(3.0),
            applied_rules: Some(vec![]),
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        };

        assert_eq!(item.manual_discount_percent, Some(10.0));
        assert_eq!(item.rule_discount_amount, Some(5.0));
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p shared order::types::tests::test_cart_item_snapshot_rule_fields
```
Expected: FAIL - missing fields

**Step 3: Update CartItemSnapshot struct**

Update `CartItemSnapshot` in `shared/src/order/types.rs`:

```rust
use super::AppliedRule;

/// Cart item snapshot - complete snapshot for event recording
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CartItemSnapshot {
    /// Product ID
    pub id: String,
    /// Instance ID (content-addressed hash)
    pub instance_id: String,
    /// Product name
    pub name: String,
    /// Final price after discounts
    pub price: f64,
    /// Original price before discounts
    pub original_price: Option<f64>,
    /// Quantity
    pub quantity: i32,
    /// Unpaid quantity (computed: quantity - paid_quantity)
    #[serde(default)]
    pub unpaid_quantity: i32,
    /// Selected options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_options: Option<Vec<ItemOption>>,
    /// Selected specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_specification: Option<SpecificationInfo>,

    // === Manual Adjustment ===
    /// Manual discount percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_discount_percent: Option<f64>,

    // === Rule Adjustments ===
    /// Rule discount amount (calculated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_discount_amount: Option<f64>,
    /// Rule surcharge amount (calculated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_surcharge_amount: Option<f64>,
    /// Applied rules list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_rules: Option<Vec<AppliedRule>>,

    // === Other ===
    /// Item note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Authorizer ID (for discounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<String>,
    /// Authorizer name snapshot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_name: Option<String>,
}
```

Also update `CartItemInput`:

```rust
/// Cart item input - for adding items (without instance_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItemInput {
    /// Product ID
    pub product_id: String,
    /// Product name
    pub name: String,
    /// Price
    pub price: f64,
    /// Original price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_price: Option<f64>,
    /// Quantity
    pub quantity: i32,
    /// Selected options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_options: Option<Vec<ItemOption>>,
    /// Selected specification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_specification: Option<SpecificationInfo>,
    /// Manual discount percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_discount_percent: Option<f64>,
    /// Item note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Authorizer ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<String>,
    /// Authorizer name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_name: Option<String>,
}
```

And update `ItemChanges`:

```rust
/// Item changes for modification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ItemChanges {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i32>,
    /// Manual discount percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_discount_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Selected options (None = no change, Some(vec) = replace options)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_options: Option<Vec<ItemOption>>,
    /// Selected specification (None = no change, Some(spec) = replace specification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_specification: Option<SpecificationInfo>,
}
```

**Step 4: Run test to verify it passes**

```bash
cargo test -p shared order::types::tests
```
Expected: PASS

**Step 5: Commit**

```bash
git add shared/src/order/types.rs
git commit -m "feat(shared): update CartItemSnapshot with rule adjustment fields"
```

---

### Task 1.4: Update OrderSnapshot with order-level rule fields

**Files:**
- Modify: `shared/src/order/snapshot.rs`

**Step 1: Add test for new fields**

Add to `shared/src/order/snapshot.rs`:

```rust
#[test]
fn test_order_snapshot_rule_fields() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.order_rule_discount_amount = Some(10.0);
    snapshot.order_rule_surcharge_amount = Some(5.0);
    snapshot.order_manual_discount_percent = Some(5.0);

    assert_eq!(snapshot.order_rule_discount_amount, Some(10.0));
    assert_eq!(snapshot.order_manual_discount_percent, Some(5.0));
}
```

**Step 2: Run test to verify it fails**

```bash
cargo test -p shared order::snapshot::test_order_snapshot_rule_fields
```
Expected: FAIL - missing fields

**Step 3: Update OrderSnapshot struct**

Add new fields to `OrderSnapshot` in `shared/src/order/snapshot.rs`:

```rust
use super::AppliedRule;

/// Order snapshot - computed from event stream
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderSnapshot {
    // ... existing fields ...

    // === Order-level Rule Adjustments ===
    /// Order-level rule discount amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_rule_discount_amount: Option<f64>,
    /// Order-level rule surcharge amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_rule_surcharge_amount: Option<f64>,
    /// Order-level applied rules
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_applied_rules: Option<Vec<AppliedRule>>,

    // === Order-level Manual Adjustments (pick one) ===
    /// Order-level manual discount percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_manual_discount_percent: Option<f64>,
    /// Order-level manual discount fixed amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_manual_discount_fixed: Option<f64>,

    // ... rest of existing fields ...
}
```

Also update `OrderSnapshot::new()` to initialize new fields to `None`.

**Step 4: Run test to verify it passes**

```bash
cargo test -p shared order::snapshot::test_order_snapshot_rule_fields
```
Expected: PASS

**Step 5: Commit**

```bash
git add shared/src/order/snapshot.rs
git commit -m "feat(shared): add order-level rule adjustment fields to OrderSnapshot"
```

---

### Task 1.5: Add ToggleRuleSkip command and event

**Files:**
- Modify: `shared/src/order/command.rs`
- Modify: `shared/src/order/event.rs`

**Step 1: Add ToggleRuleSkip to OrderCommandPayload**

Add to `shared/src/order/command.rs`:

```rust
/// Toggle rule skip status
ToggleRuleSkip {
    order_id: String,
    rule_id: String,
    skipped: bool,
},
```

Also add to `target_order_id()` match:
```rust
OrderCommandPayload::ToggleRuleSkip { order_id, .. } => Some(order_id),
```

**Step 2: Add RuleSkipToggled event**

Add to `shared/src/order/event.rs`:

```rust
// In OrderEventType enum
RuleSkipToggled,

// In EventPayload enum
RuleSkipToggled {
    rule_id: String,
    skipped: bool,
    /// Recalculated amounts after toggle
    subtotal: f64,
    discount: f64,
    surcharge: f64,
    total: f64,
},
```

Also add to `Display` impl for `OrderEventType`:
```rust
OrderEventType::RuleSkipToggled => write!(f, "RULE_SKIP_TOGGLED"),
```

**Step 3: Run compilation check**

```bash
cargo check -p shared
```
Expected: PASS

**Step 4: Commit**

```bash
git add shared/src/order/command.rs shared/src/order/event.rs
git commit -m "feat(shared): add ToggleRuleSkip command and RuleSkipToggled event"
```

---

## Phase 2: Implement Calculation Engine

### Task 2.1: Create price calculation module

**Files:**
- Create: `edge-server/src/pricing/item_calculator.rs`
- Modify: `edge-server/src/pricing/mod.rs`

**Step 1: Create item_calculator.rs with tests**

Create `edge-server/src/pricing/item_calculator.rs`:

```rust
//! Item-level price calculation
//!
//! Calculates final price for a single item applying:
//! 1. Manual discount (based on base price)
//! 2. Rule discounts (stackable/exclusive, based on after-manual price)
//! 3. Rule surcharges (stackable/exclusive, based on base price)

use crate::db::models::PriceRule;
use rust_decimal::prelude::*;
use shared::order::AppliedRule;
use shared::models::price_rule::{AdjustmentType, RuleType};

/// Result of item price calculation
#[derive(Debug, Clone)]
pub struct ItemCalculationResult {
    pub base: f64,
    pub manual_discount_amount: f64,
    pub after_manual: f64,
    pub rule_discount_amount: f64,
    pub after_discount: f64,
    pub rule_surcharge_amount: f64,
    pub item_final: f64,
    pub applied_rules: Vec<AppliedRule>,
}

/// Calculate effective priority for a rule
pub fn calculate_effective_priority(rule: &PriceRule) -> i32 {
    let zone_weight = match rule.zone_scope {
        -1 => 0,  // Global
        0 => 1,   // Retail
        _ => 2,   // Specific zone
    };

    let product_weight = match rule.product_scope {
        shared::models::price_rule::ProductScope::Global => 0,
        shared::models::price_rule::ProductScope::Category => 1,
        shared::models::price_rule::ProductScope::Tag => 2,
        shared::models::price_rule::ProductScope::Product => 3,
    };

    (zone_weight * 10 + product_weight) * 1000 + rule.priority
}

/// Calculate item price with all adjustments
pub fn calculate_item_price(
    original_price: f64,
    options_modifier: f64,
    manual_discount_percent: f64,
    matched_rules: &[&PriceRule],
) -> ItemCalculationResult {
    let base = to_decimal(original_price) + to_decimal(options_modifier);

    // Step 1: Manual discount
    let manual_discount_rate = to_decimal(manual_discount_percent) / dec!(100);
    let manual_discount_amount = base * manual_discount_rate;
    let after_manual = base - manual_discount_amount;

    // Step 2: Rule discounts
    let discount_rules: Vec<_> = matched_rules
        .iter()
        .filter(|r| matches!(r.rule_type, RuleType::Discount))
        .copied()
        .collect();

    let (rule_discount_amount, discount_applied) =
        apply_discount_rules(&discount_rules, after_manual);
    let after_discount = (after_manual - rule_discount_amount).max(Decimal::ZERO);

    // Step 3: Rule surcharges (based on base)
    let surcharge_rules: Vec<_> = matched_rules
        .iter()
        .filter(|r| matches!(r.rule_type, RuleType::Surcharge))
        .copied()
        .collect();

    let (rule_surcharge_amount, surcharge_applied) =
        apply_surcharge_rules(&surcharge_rules, base);

    // Step 4: Final price
    let item_final = (after_discount + rule_surcharge_amount).max(Decimal::ZERO);

    // Combine applied rules
    let mut applied_rules = discount_applied;
    applied_rules.extend(surcharge_applied);

    ItemCalculationResult {
        base: to_f64(base),
        manual_discount_amount: to_f64(manual_discount_amount),
        after_manual: to_f64(after_manual),
        rule_discount_amount: to_f64(rule_discount_amount),
        after_discount: to_f64(after_discount),
        rule_surcharge_amount: to_f64(rule_surcharge_amount),
        item_final: to_f64(item_final),
        applied_rules,
    }
}

fn apply_discount_rules(
    rules: &[&PriceRule],
    base_amount: Decimal,
) -> (Decimal, Vec<AppliedRule>) {
    if rules.is_empty() {
        return (Decimal::ZERO, vec![]);
    }

    // Check for exclusive rules first
    let exclusive: Vec<_> = rules.iter()
        .filter(|r| r.is_exclusive)
        .copied()
        .collect();

    if !exclusive.is_empty() {
        let winner = select_winner(&exclusive);
        let amount = calculate_single_adjustment(winner, base_amount);
        let applied = AppliedRule::from_rule(winner, to_f64(amount));
        return (amount, vec![applied]);
    }

    // Normal processing
    let non_stackable: Vec<_> = rules.iter()
        .filter(|r| !r.is_stackable)
        .copied()
        .collect();
    let stackable: Vec<_> = rules.iter()
        .filter(|r| r.is_stackable)
        .copied()
        .collect();

    let mut total_amount = Decimal::ZERO;
    let mut applied_rules = vec![];
    let mut current_base = base_amount;

    // Non-stackable winner
    if !non_stackable.is_empty() {
        let winner = select_winner(&non_stackable);
        if matches!(winner.adjustment_type, AdjustmentType::Percentage) {
            let rate = Decimal::from(winner.adjustment_value) / dec!(100);
            let amount = current_base * rate;
            current_base = current_base * (Decimal::ONE - rate);
            total_amount += amount;
            applied_rules.push(AppliedRule::from_rule(winner, to_f64(amount)));
        } else {
            let amount = Decimal::from(winner.adjustment_value) / dec!(100);
            total_amount += amount;
            applied_rules.push(AppliedRule::from_rule(winner, to_f64(amount)));
        }
    }

    // Stackable rules (percentage multiply, fixed add)
    for rule in &stackable {
        if matches!(rule.adjustment_type, AdjustmentType::Percentage) {
            let rate = Decimal::from(rule.adjustment_value) / dec!(100);
            let amount = current_base * rate;
            current_base = current_base * (Decimal::ONE - rate);
            total_amount += amount;
            applied_rules.push(AppliedRule::from_rule(rule, to_f64(amount)));
        } else {
            let amount = Decimal::from(rule.adjustment_value) / dec!(100);
            total_amount += amount;
            applied_rules.push(AppliedRule::from_rule(rule, to_f64(amount)));
        }
    }

    (total_amount, applied_rules)
}

fn apply_surcharge_rules(
    rules: &[&PriceRule],
    base_amount: Decimal,
) -> (Decimal, Vec<AppliedRule>) {
    if rules.is_empty() {
        return (Decimal::ZERO, vec![]);
    }

    // Check for exclusive rules first
    let exclusive: Vec<_> = rules.iter()
        .filter(|r| r.is_exclusive)
        .copied()
        .collect();

    if !exclusive.is_empty() {
        let winner = select_winner(&exclusive);
        let amount = calculate_single_surcharge(winner, base_amount);
        let applied = AppliedRule::from_rule(winner, to_f64(amount));
        return (amount, vec![applied]);
    }

    // Normal processing
    let non_stackable: Vec<_> = rules.iter()
        .filter(|r| !r.is_stackable)
        .copied()
        .collect();
    let stackable: Vec<_> = rules.iter()
        .filter(|r| r.is_stackable)
        .copied()
        .collect();

    let mut multiplier = Decimal::ONE;
    let mut fixed_total = Decimal::ZERO;
    let mut applied_rules = vec![];

    // Non-stackable winner
    if !non_stackable.is_empty() {
        let winner = select_winner(&non_stackable);
        if matches!(winner.adjustment_type, AdjustmentType::Percentage) {
            let rate = Decimal::from(winner.adjustment_value) / dec!(100);
            multiplier *= Decimal::ONE + rate;
            let amount = base_amount * rate;
            applied_rules.push(AppliedRule::from_rule(winner, to_f64(amount)));
        } else {
            let amount = Decimal::from(winner.adjustment_value) / dec!(100);
            fixed_total += amount;
            applied_rules.push(AppliedRule::from_rule(winner, to_f64(amount)));
        }
    }

    // Stackable rules
    for rule in &stackable {
        if matches!(rule.adjustment_type, AdjustmentType::Percentage) {
            let rate = Decimal::from(rule.adjustment_value) / dec!(100);
            multiplier *= Decimal::ONE + rate;
            let amount = base_amount * rate;
            applied_rules.push(AppliedRule::from_rule(rule, to_f64(amount)));
        } else {
            let amount = Decimal::from(rule.adjustment_value) / dec!(100);
            fixed_total += amount;
            applied_rules.push(AppliedRule::from_rule(rule, to_f64(amount)));
        }
    }

    let percent_amount = base_amount * multiplier - base_amount;
    (percent_amount + fixed_total, applied_rules)
}

fn select_winner<'a>(rules: &[&'a PriceRule]) -> &'a PriceRule {
    rules.iter()
        .max_by(|a, b| {
            let pa = calculate_effective_priority(a);
            let pb = calculate_effective_priority(b);
            pa.cmp(&pb).then_with(|| a.created_at.cmp(&b.created_at))
        })
        .unwrap()
}

fn calculate_single_adjustment(rule: &PriceRule, base: Decimal) -> Decimal {
    match rule.adjustment_type {
        AdjustmentType::Percentage => {
            base * Decimal::from(rule.adjustment_value) / dec!(100)
        }
        AdjustmentType::FixedAmount => {
            Decimal::from(rule.adjustment_value) / dec!(100)
        }
    }
}

fn calculate_single_surcharge(rule: &PriceRule, base: Decimal) -> Decimal {
    calculate_single_adjustment(rule, base)
}

fn to_decimal(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or_default()
}

fn to_f64(value: Decimal) -> f64 {
    value
        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
        .to_f64()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::price_rule::{ProductScope, RuleType, AdjustmentType};

    fn make_rule(
        id: &str,
        rule_type: RuleType,
        adj_type: AdjustmentType,
        value: i32,
        priority: i32,
        stackable: bool,
        exclusive: bool,
    ) -> PriceRule {
        PriceRule {
            id: Some(id.to_string()),
            name: id.to_string(),
            display_name: id.to_string(),
            receipt_name: id.to_string(),
            description: None,
            rule_type,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: adj_type,
            adjustment_value: value,
            priority,
            is_stackable: stackable,
            is_exclusive: exclusive,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 0,
        }
    }

    #[test]
    fn test_simple_discount() {
        let rule = make_rule(
            "r1",
            RuleType::Discount,
            AdjustmentType::Percentage,
            10,
            0,
            true,
            false,
        );

        let result = calculate_item_price(100.0, 0.0, 0.0, &[&rule]);

        assert_eq!(result.base, 100.0);
        assert_eq!(result.rule_discount_amount, 10.0);
        assert_eq!(result.item_final, 90.0);
    }

    #[test]
    fn test_manual_then_rule_discount() {
        let rule = make_rule(
            "r1",
            RuleType::Discount,
            AdjustmentType::Percentage,
            10,
            0,
            true,
            false,
        );

        // base=100, manual 10% -> 90, rule 10% of 90 = 9 -> 81
        let result = calculate_item_price(100.0, 0.0, 10.0, &[&rule]);

        assert_eq!(result.base, 100.0);
        assert_eq!(result.manual_discount_amount, 10.0);
        assert_eq!(result.after_manual, 90.0);
        assert_eq!(result.rule_discount_amount, 9.0);
        assert_eq!(result.item_final, 81.0);
    }

    #[test]
    fn test_exclusive_wins() {
        let r1 = make_rule(
            "r1",
            RuleType::Discount,
            AdjustmentType::Percentage,
            50, // 50% exclusive
            10,
            false,
            true,
        );
        let r2 = make_rule(
            "r2",
            RuleType::Discount,
            AdjustmentType::Percentage,
            10,
            5,
            true,
            false,
        );

        let result = calculate_item_price(100.0, 0.0, 0.0, &[&r1, &r2]);

        // Exclusive wins, only 50% applied
        assert_eq!(result.rule_discount_amount, 50.0);
        assert_eq!(result.item_final, 50.0);
        assert_eq!(result.applied_rules.len(), 1);
    }

    #[test]
    fn test_surcharge_based_on_base() {
        let discount = make_rule(
            "d1",
            RuleType::Discount,
            AdjustmentType::Percentage,
            10,
            0,
            true,
            false,
        );
        let surcharge = make_rule(
            "s1",
            RuleType::Surcharge,
            AdjustmentType::Percentage,
            10,
            0,
            true,
            false,
        );

        // base=100, discount 10% -> 90, surcharge 10% of base=10 -> 100
        let result = calculate_item_price(100.0, 0.0, 0.0, &[&discount, &surcharge]);

        assert_eq!(result.rule_discount_amount, 10.0);
        assert_eq!(result.rule_surcharge_amount, 10.0);
        assert_eq!(result.item_final, 100.0);
    }
}
```

**Step 2: Update mod.rs**

Add to `edge-server/src/pricing/mod.rs`:

```rust
mod item_calculator;
pub use item_calculator::*;
```

**Step 3: Run tests**

```bash
cargo test -p edge-server pricing::item_calculator::tests
```
Expected: PASS

**Step 4: Commit**

```bash
git add edge-server/src/pricing/item_calculator.rs edge-server/src/pricing/mod.rs
git commit -m "feat(pricing): add item-level price calculator with stackable/exclusive logic"
```

---

### Task 2.2: Create order-level calculator

**Files:**
- Create: `edge-server/src/pricing/order_calculator.rs`
- Modify: `edge-server/src/pricing/mod.rs`

**Step 1: Create order_calculator.rs**

Create `edge-server/src/pricing/order_calculator.rs`:

```rust
//! Order-level price calculation

use crate::db::models::PriceRule;
use rust_decimal::prelude::*;
use shared::order::AppliedRule;
use super::item_calculator::{apply_discount_rules, apply_surcharge_rules, to_decimal, to_f64};

/// Result of order price calculation
#[derive(Debug, Clone)]
pub struct OrderCalculationResult {
    pub subtotal: f64,
    pub order_rule_discount_amount: f64,
    pub after_order_rule_discount: f64,
    pub order_rule_surcharge_amount: f64,
    pub after_order_rule: f64,
    pub order_manual_discount_amount: f64,
    pub total: f64,
    pub order_applied_rules: Vec<AppliedRule>,
}

/// Calculate order-level price adjustments
pub fn calculate_order_price(
    subtotal: f64,
    order_rules: &[&PriceRule],
    manual_discount_percent: Option<f64>,
    manual_discount_fixed: Option<f64>,
) -> OrderCalculationResult {
    let subtotal_dec = to_decimal(subtotal);

    // Step 1: Order rule discounts
    let discount_rules: Vec<_> = order_rules
        .iter()
        .filter(|r| matches!(r.rule_type, shared::models::price_rule::RuleType::Discount))
        .copied()
        .collect();

    let (rule_discount_amount, mut applied_rules) =
        apply_discount_rules(&discount_rules, subtotal_dec);
    let after_order_rule_discount = (subtotal_dec - rule_discount_amount).max(Decimal::ZERO);

    // Step 2: Order rule surcharges
    let surcharge_rules: Vec<_> = order_rules
        .iter()
        .filter(|r| matches!(r.rule_type, shared::models::price_rule::RuleType::Surcharge))
        .copied()
        .collect();

    let (rule_surcharge_amount, surcharge_applied) =
        apply_surcharge_rules(&surcharge_rules, subtotal_dec);
    applied_rules.extend(surcharge_applied);

    let after_order_rule = after_order_rule_discount + rule_surcharge_amount;

    // Step 3: Manual discount (based on after_order_rule)
    let manual_discount_amount = if let Some(percent) = manual_discount_percent {
        after_order_rule * to_decimal(percent) / dec!(100)
    } else if let Some(fixed) = manual_discount_fixed {
        to_decimal(fixed)
    } else {
        Decimal::ZERO
    };

    let total = (after_order_rule - manual_discount_amount).max(Decimal::ZERO);

    OrderCalculationResult {
        subtotal,
        order_rule_discount_amount: to_f64(rule_discount_amount),
        after_order_rule_discount: to_f64(after_order_rule_discount),
        order_rule_surcharge_amount: to_f64(rule_surcharge_amount),
        after_order_rule: to_f64(after_order_rule),
        order_manual_discount_amount: to_f64(manual_discount_amount),
        total: to_f64(total),
        order_applied_rules: applied_rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::price_rule::{ProductScope, RuleType, AdjustmentType};
    use crate::db::models::PriceRule;

    fn make_order_rule(
        id: &str,
        rule_type: RuleType,
        adj_type: AdjustmentType,
        value: i32,
    ) -> PriceRule {
        PriceRule {
            id: Some(id.to_string()),
            name: id.to_string(),
            display_name: id.to_string(),
            receipt_name: id.to_string(),
            description: None,
            rule_type,
            product_scope: ProductScope::Global,
            target: None,
            zone_scope: -1,
            adjustment_type: adj_type,
            adjustment_value: value,
            priority: 0,
            is_stackable: true,
            is_exclusive: false,
            valid_from: None,
            valid_until: None,
            active_days: None,
            active_start_time: None,
            active_end_time: None,
            is_active: true,
            created_by: None,
            created_at: 0,
        }
    }

    #[test]
    fn test_order_discount() {
        let rule = make_order_rule(
            "order-discount",
            RuleType::Discount,
            AdjustmentType::Percentage,
            10,
        );

        let result = calculate_order_price(100.0, &[&rule], None, None);

        assert_eq!(result.subtotal, 100.0);
        assert_eq!(result.order_rule_discount_amount, 10.0);
        assert_eq!(result.total, 90.0);
    }

    #[test]
    fn test_order_manual_discount_percent() {
        let result = calculate_order_price(100.0, &[], Some(10.0), None);

        assert_eq!(result.order_manual_discount_amount, 10.0);
        assert_eq!(result.total, 90.0);
    }

    #[test]
    fn test_order_manual_discount_fixed() {
        let result = calculate_order_price(100.0, &[], None, Some(15.0));

        assert_eq!(result.order_manual_discount_amount, 15.0);
        assert_eq!(result.total, 85.0);
    }
}
```

**Step 2: Update mod.rs**

Add to `edge-server/src/pricing/mod.rs`:

```rust
mod order_calculator;
pub use order_calculator::*;
```

**Step 3: Run tests**

```bash
cargo test -p edge-server pricing::order_calculator::tests
```
Expected: PASS

**Step 4: Commit**

```bash
git add edge-server/src/pricing/order_calculator.rs edge-server/src/pricing/mod.rs
git commit -m "feat(pricing): add order-level price calculator"
```

---

## Phase 3: Rule Loading and Caching

### Task 3.1: Add rule cache to OrdersManager

**Files:**
- Modify: `edge-server/src/orders/manager.rs`

**Step 1: Add rule cache field**

Add to `OrdersManager` struct:

```rust
use crate::db::models::PriceRule;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct OrdersManager {
    storage: OrderStorage,
    event_tx: broadcast::Sender<OrderEvent>,
    epoch: String,
    /// Cached rules per order
    rule_cache: RwLock<HashMap<String, Vec<PriceRule>>>,
}
```

Update `new()` and `with_storage()` to initialize:
```rust
rule_cache: RwLock::new(HashMap::new()),
```

**Step 2: Add cache methods**

```rust
impl OrdersManager {
    /// Cache rules for an order
    pub fn cache_rules(&self, order_id: &str, rules: Vec<PriceRule>) {
        let mut cache = self.rule_cache.write().unwrap();
        cache.insert(order_id.to_string(), rules);
    }

    /// Get cached rules for an order
    pub fn get_cached_rules(&self, order_id: &str) -> Option<Vec<PriceRule>> {
        let cache = self.rule_cache.read().unwrap();
        cache.get(order_id).cloned()
    }

    /// Remove cached rules for an order
    pub fn remove_cached_rules(&self, order_id: &str) {
        let mut cache = self.rule_cache.write().unwrap();
        cache.remove(order_id);
    }
}
```

**Step 3: Clear cache on order completion/void**

In `process_command`, after processing `CompleteOrder` or `VoidOrder`, call:
```rust
self.remove_cached_rules(&order_id);
```

**Step 4: Run existing tests**

```bash
cargo test -p edge-server orders::manager::tests
```
Expected: PASS (existing tests should still pass)

**Step 5: Commit**

```bash
git add edge-server/src/orders/manager.rs
git commit -m "feat(orders): add rule cache to OrdersManager"
```

---

## Phase 4: Integrate with Commands

### Task 4.1: Update AddItems to apply rules

**Files:**
- Modify: `edge-server/src/orders/actions/add_items.rs`
- Modify: `edge-server/src/orders/reducer.rs`

This task requires significant changes to integrate the price calculation into the add_items flow. The implementation should:

1. Load rules from cache (or database if not cached)
2. For each item, calculate price using `calculate_item_price`
3. Store applied rules in the snapshot

**Step 1: Update input_to_snapshot in reducer.rs**

The function should now accept rules and calculate prices:

```rust
pub fn input_to_snapshot_with_rules(
    input: &CartItemInput,
    rules: &[&PriceRule],
) -> CartItemSnapshot {
    let options_modifier: f64 = input
        .selected_options
        .as_ref()
        .map(|opts| opts.iter().filter_map(|o| o.price_modifier).sum())
        .unwrap_or(0.0);

    let manual_discount = input.manual_discount_percent.unwrap_or(0.0);

    let calc_result = calculate_item_price(
        input.original_price.unwrap_or(input.price),
        options_modifier,
        manual_discount,
        rules,
    );

    CartItemSnapshot {
        id: input.product_id.clone(),
        instance_id: generate_instance_id(input),
        name: input.name.clone(),
        price: calc_result.item_final,
        original_price: input.original_price,
        quantity: input.quantity,
        unpaid_quantity: input.quantity,
        selected_options: input.selected_options.clone(),
        selected_specification: input.selected_specification.clone(),
        manual_discount_percent: input.manual_discount_percent,
        rule_discount_amount: Some(calc_result.rule_discount_amount),
        rule_surcharge_amount: Some(calc_result.rule_surcharge_amount),
        applied_rules: Some(calc_result.applied_rules),
        note: input.note.clone(),
        authorizer_id: input.authorizer_id.clone(),
        authorizer_name: input.authorizer_name.clone(),
    }
}
```

**Step 2: Update tests**

```bash
cargo test -p edge-server orders::actions::add_items::tests
```

**Step 3: Commit**

```bash
git add edge-server/src/orders/actions/add_items.rs edge-server/src/orders/reducer.rs
git commit -m "feat(orders): integrate price calculation into AddItems"
```

---

### Task 4.2: Add ToggleRuleSkip action

**Files:**
- Create: `edge-server/src/orders/actions/toggle_rule_skip.rs`
- Modify: `edge-server/src/orders/actions/mod.rs`

**Step 1: Create toggle_rule_skip.rs**

```rust
//! ToggleRuleSkip command handler

use async_trait::async_trait;
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

#[derive(Debug, Clone)]
pub struct ToggleRuleSkipAction {
    pub order_id: String,
    pub rule_id: String,
    pub skipped: bool,
}

#[async_trait]
impl CommandHandler for ToggleRuleSkipAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Load snapshot
        let mut snapshot = ctx.load_snapshot(&self.order_id)?;

        // 2. Validate status
        if !matches!(snapshot.status, OrderStatus::Active) {
            return Err(OrderError::InvalidOperation(
                "Cannot toggle rule on non-active order".to_string()
            ));
        }

        // 3. Find and toggle rule in items
        for item in &mut snapshot.items {
            if let Some(ref mut rules) = item.applied_rules {
                for rule in rules.iter_mut() {
                    if rule.rule_id == self.rule_id {
                        rule.skipped = self.skipped;
                    }
                }
            }
        }

        // 4. Toggle in order-level rules
        if let Some(ref mut rules) = snapshot.order_applied_rules {
            for rule in rules.iter_mut() {
                if rule.rule_id == self.rule_id {
                    rule.skipped = self.skipped;
                }
            }
        }

        // 5. Recalculate totals (simplified - full impl needs price recalc)
        // TODO: Implement full recalculation

        // 6. Generate event
        let seq = ctx.next_sequence();
        let event = OrderEvent::new(
            seq,
            self.order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp),
            OrderEventType::RuleSkipToggled,
            EventPayload::RuleSkipToggled {
                rule_id: self.rule_id.clone(),
                skipped: self.skipped,
                subtotal: snapshot.subtotal,
                discount: snapshot.discount,
                surcharge: snapshot.surcharge,
                total: snapshot.total,
            },
        );

        Ok(vec![event])
    }
}
```

**Step 2: Update mod.rs**

Add to `edge-server/src/orders/actions/mod.rs`:

```rust
mod toggle_rule_skip;
pub use toggle_rule_skip::ToggleRuleSkipAction;

// In CommandAction enum:
ToggleRuleSkip(ToggleRuleSkipAction),

// In execute() match:
CommandAction::ToggleRuleSkip(action) => action.execute(ctx, metadata).await,

// In from_command():
OrderCommandPayload::ToggleRuleSkip { order_id, rule_id, skipped } => {
    CommandAction::ToggleRuleSkip(ToggleRuleSkipAction {
        order_id: order_id.clone(),
        rule_id: rule_id.clone(),
        skipped: *skipped,
    })
}
```

**Step 3: Run tests**

```bash
cargo test -p edge-server orders::actions
```

**Step 4: Commit**

```bash
git add edge-server/src/orders/actions/toggle_rule_skip.rs edge-server/src/orders/actions/mod.rs
git commit -m "feat(orders): add ToggleRuleSkip action"
```

---

## Phase 5: Update Time Matcher

### Task 5.1: Update matcher for new time fields

**Files:**
- Modify: `edge-server/src/pricing/matcher.rs`

**Step 1: Update is_time_valid function**

```rust
/// Check if rule is valid at the given timestamp
pub fn is_time_valid(rule: &PriceRule, current_time: i64) -> bool {
    // Check valid_from/valid_until
    if let Some(from) = rule.valid_from {
        if current_time < from {
            return false;
        }
    }

    if let Some(until) = rule.valid_until {
        if current_time > until {
            return false;
        }
    }

    // Check active_days
    if let Some(ref days) = rule.active_days {
        let datetime = chrono::DateTime::from_timestamp_millis(current_time)
            .unwrap_or_else(|| chrono::Utc::now());
        let weekday = datetime.weekday().num_days_from_sunday() as u8;
        if !days.contains(&weekday) {
            return false;
        }
    }

    // Check active_start_time/active_end_time
    if rule.active_start_time.is_some() || rule.active_end_time.is_some() {
        let datetime = chrono::DateTime::from_timestamp_millis(current_time)
            .unwrap_or_else(|| chrono::Utc::now());
        let current_time_str = datetime.format("%H:%M").to_string();

        if let Some(ref start) = rule.active_start_time {
            if current_time_str < *start {
                return false;
            }
        }

        if let Some(ref end) = rule.active_end_time {
            if current_time_str > *end {
                return false;
            }
        }
    }

    true
}
```

**Step 2: Add tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_from_future() {
        let mut rule = create_test_rule();
        rule.valid_from = Some(i64::MAX); // Far future

        assert!(!is_time_valid(&rule, chrono::Utc::now().timestamp_millis()));
    }

    #[test]
    fn test_valid_until_past() {
        let mut rule = create_test_rule();
        rule.valid_until = Some(0); // Past

        assert!(!is_time_valid(&rule, chrono::Utc::now().timestamp_millis()));
    }

    #[test]
    fn test_active_days() {
        let mut rule = create_test_rule();
        rule.active_days = Some(vec![1, 2, 3, 4, 5]); // Mon-Fri

        // This test is time-dependent, so we just verify it doesn't crash
        let _ = is_time_valid(&rule, chrono::Utc::now().timestamp_millis());
    }
}
```

**Step 3: Run tests**

```bash
cargo test -p edge-server pricing::matcher::tests
```

**Step 4: Commit**

```bash
git add edge-server/src/pricing/matcher.rs
git commit -m "feat(pricing): update time matcher for new time control fields"
```

---

## Summary

This plan covers:

1. **Phase 1**: Data structure updates (PriceRule, AppliedRule, CartItemSnapshot, OrderSnapshot, Commands/Events)
2. **Phase 2**: Calculation engine (item-level and order-level)
3. **Phase 3**: Rule caching in OrdersManager
4. **Phase 4**: Integration with AddItems and ToggleRuleSkip
5. **Phase 5**: Time matcher updates

Each task follows TDD: write failing test → implement → verify → commit.

---

**Plan complete and saved to `docs/plans/2026-01-22-price-rule-implementation-plan.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
