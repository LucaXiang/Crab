//! Order snapshot - computed state from event stream
//!
//! The snapshot includes a `state_checksum` field for drift detection.
//! Clients can compare their locally computed checksum with the server's
//! to detect if the reducer logic has diverged.

use super::types::{CartItemSnapshot, CompRecord, LossReason, PaymentRecord, ServiceType, VoidType};
use super::AppliedRule;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Order status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    #[default]
    Active,
    Completed,
    Void,
    Moved,
    Merged,
}

/// Order snapshot - computed from event stream
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderSnapshot {
    /// Order ID (assigned by server)
    pub order_id: String,
    /// Table ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
    /// Table name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    /// Zone ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_id: Option<String>,
    /// Zone name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_name: Option<String>,
    /// Guest count
    pub guest_count: i32,
    /// Whether this is a retail order
    #[serde(default)]
    pub is_retail: bool,
    /// Service type (dine-in or takeout, for retail orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_type: Option<ServiceType>,
    /// Queue number (server-generated, for retail orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_number: Option<u32>,
    /// Order status
    pub status: OrderStatus,

    // === Void Information (only when status == Void) ===
    /// Void type (CANCELLED or LOSS_SETTLED)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub void_type: Option<VoidType>,
    /// Loss reason (only for LOSS_SETTLED)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loss_reason: Option<LossReason>,
    /// Loss amount (only for LOSS_SETTLED, equals remaining_amount at void time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loss_amount: Option<f64>,
    /// Void note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub void_note: Option<String>,
    /// Items in the order
    pub items: Vec<CartItemSnapshot>,
    /// Comp records (audit trail for comped items)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comps: Vec<CompRecord>,
    /// Payment records
    pub payments: Vec<PaymentRecord>,
    // === Financial Totals (all computed by server) ===
    /// Original total before any discounts/surcharges
    /// = Σ((original_price ?? price) * quantity)
    #[serde(default)]
    pub original_total: f64,
    /// Subtotal after item-level adjustments (before order-level adjustments)
    pub subtotal: f64,
    /// Total discount amount (item-level + order-level)
    #[serde(default)]
    pub total_discount: f64,
    /// Total surcharge amount (item-level + order-level)
    #[serde(default)]
    pub total_surcharge: f64,
    /// Tax amount
    #[serde(default)]
    pub tax: f64,
    /// Total discount amount
    #[serde(default)]
    pub discount: f64,
    /// Total amount to pay
    pub total: f64,
    /// Amount already paid
    #[serde(default)]
    pub paid_amount: f64,
    /// Remaining amount to pay (total - paid_amount)
    #[serde(default)]
    pub remaining_amount: f64,
    /// Quantities paid per item (for split bill)
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub paid_item_quantities: std::collections::HashMap<String, i32>,
    /// Whether this order has amount-based split payments (金额分单)
    /// If true, item-based split is disabled
    #[serde(default)]
    pub has_amount_split: bool,
    /// AA split: total number of shares (锁定的人数)
    /// Set on first AA payment, locked afterwards
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aa_total_shares: Option<i32>,
    /// AA split: number of shares already paid
    #[serde(default)]
    pub aa_paid_shares: i32,
    /// Receipt number (server-generated at OpenTable)
    pub receipt_number: String,
    /// Whether this is a pre-payment order
    #[serde(default)]
    pub is_pre_payment: bool,
    /// Order-level note (覆盖式，None = 无备注)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

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
    /// Order-level manual surcharge percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_manual_surcharge_percent: Option<f64>,
    /// Order-level manual surcharge fixed amount
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_manual_surcharge_fixed: Option<f64>,

    /// Order start time
    pub start_time: i64,
    /// Order end time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<i64>,
    /// Creation timestamp
    pub created_at: i64,
    /// Last update timestamp
    pub updated_at: i64,
    /// Last applied event sequence (for incremental updates)
    pub last_sequence: u64,
    /// State checksum for drift detection (hex string)
    /// Computed from: items.len, total, paid_amount, last_sequence, status
    /// Clients should compare their computed checksum with this value
    /// to detect reducer drift and trigger full sync if needed
    #[serde(default)]
    pub state_checksum: String,
}

impl OrderSnapshot {
    /// Create a new empty order
    pub fn new(order_id: String) -> Self {
        let now = crate::util::now_millis();
        let mut snapshot = Self {
            order_id,
            table_id: None,
            table_name: None,
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
            service_type: None,
            queue_number: None,
            status: OrderStatus::Active,
            void_type: None,
            loss_reason: None,
            loss_amount: None,
            void_note: None,
            items: Vec::new(),
            comps: Vec::new(),
            payments: Vec::new(),
            original_total: 0.0,
            subtotal: 0.0,
            total_discount: 0.0,
            total_surcharge: 0.0,
            tax: 0.0,
            discount: 0.0,
            total: 0.0,
            paid_amount: 0.0,
            remaining_amount: 0.0,
            paid_item_quantities: std::collections::HashMap::new(),
            has_amount_split: false,
            aa_total_shares: None,
            aa_paid_shares: 0,
            receipt_number: String::new(),
            is_pre_payment: false,
            note: None,
            order_rule_discount_amount: None,
            order_rule_surcharge_amount: None,
            order_applied_rules: None,
            order_manual_discount_percent: None,
            order_manual_discount_fixed: None,
            order_manual_surcharge_percent: None,
            order_manual_surcharge_fixed: None,
            start_time: now,
            end_time: None,
            created_at: now,
            updated_at: now,
            last_sequence: 0,
            state_checksum: String::new(),
        };
        snapshot.update_checksum();
        snapshot
    }

    /// Check if order is active
    pub fn is_active(&self) -> bool {
        self.status == OrderStatus::Active
    }

    /// Check if order is completed
    pub fn is_completed(&self) -> bool {
        self.status == OrderStatus::Completed
    }

    /// Check if order is voided
    pub fn is_voided(&self) -> bool {
        self.status == OrderStatus::Void
    }

    /// Get active (non-removed) items
    pub fn active_items(&self) -> impl Iterator<Item = &CartItemSnapshot> {
        self.items.iter()
    }

    /// Calculate remaining amount to pay
    pub fn remaining_amount(&self) -> f64 {
        (self.total - self.paid_amount).max(0.0)
    }

    /// Check if fully paid
    pub fn is_fully_paid(&self) -> bool {
        self.paid_amount >= self.total
    }

    /// Compute state checksum for drift detection
    ///
    /// The checksum is computed from key state fields that should match
    /// between server and client after applying the same events.
    /// Returns a 16-character hex string.
    ///
    /// Fields included:
    /// - items.len() - number of items
    /// - total (cents) - order total in cents to avoid float precision issues
    /// - paid_amount (cents) - paid amount in cents
    /// - last_sequence - last applied event sequence
    /// - status - order status discriminant
    pub fn compute_checksum(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher as _;

        let mut hasher = DefaultHasher::new();

        // Hash item count
        self.items.len().hash(&mut hasher);

        // Hash total in cents (avoid float precision issues)
        ((self.total * 100.0).round() as i64).hash(&mut hasher);

        // Hash paid_amount in cents
        ((self.paid_amount * 100.0).round() as i64).hash(&mut hasher);

        // Hash last sequence
        self.last_sequence.hash(&mut hasher);

        // Hash status discriminant
        (self.status as u8).hash(&mut hasher);

        // Return as hex string
        format!("{:016x}", hasher.finish())
    }

    /// Update the state_checksum field based on current state
    pub fn update_checksum(&mut self) {
        self.state_checksum = self.compute_checksum();
    }

    /// Verify that the state_checksum matches computed checksum
    /// Returns true if checksum matches, false if drift detected
    pub fn verify_checksum(&self) -> bool {
        self.state_checksum == self.compute_checksum()
    }
}

impl Default for OrderSnapshot {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_snapshot_rule_fields() {
        let mut snapshot = OrderSnapshot::new("order-1".to_string());
        snapshot.order_rule_discount_amount = Some(10.0);
        snapshot.order_rule_surcharge_amount = Some(5.0);
        snapshot.order_manual_discount_percent = Some(5.0);

        assert_eq!(snapshot.order_rule_discount_amount, Some(10.0));
        assert_eq!(snapshot.order_rule_surcharge_amount, Some(5.0));
        assert_eq!(snapshot.order_manual_discount_percent, Some(5.0));
    }

    #[test]
    fn test_void_type_serde_roundtrip() {
        use super::super::types::VoidType;

        // 1. Create an Active snapshot (void_type = None)
        let snapshot = OrderSnapshot::new("test-void".to_string());
        let json = serde_json::to_string(&snapshot).unwrap();

        // void_type should NOT be in the JSON (skip_serializing_if)
        assert!(
            !json.contains("void_type"),
            "Active snapshot should not contain void_type in JSON"
        );

        // 2. Deserialize back - should work even without void_type key
        let restored: Result<OrderSnapshot, _> = serde_json::from_str(&json);
        assert!(
            restored.is_ok(),
            "Deserialization should succeed even without void_type key: {:?}",
            restored.err()
        );
        let restored = restored.unwrap();
        assert_eq!(restored.void_type, None);

        // 3. Create a Void snapshot with void_type set
        let mut void_snapshot = OrderSnapshot::new("test-void-2".to_string());
        void_snapshot.status = OrderStatus::Void;
        void_snapshot.void_type = Some(VoidType::LossSettled);

        let json2 = serde_json::to_string(&void_snapshot).unwrap();
        assert!(
            json2.contains("LOSS_SETTLED"),
            "Void snapshot JSON should contain LOSS_SETTLED, got: {}",
            json2
        );

        // 4. Deserialize back
        let restored2: OrderSnapshot = serde_json::from_str(&json2).unwrap();
        assert_eq!(restored2.void_type, Some(VoidType::LossSettled));
    }

    #[test]
    fn test_void_type_to_string_conversion() {
        use super::super::types::{LossReason, VoidType};

        // Test the exact conversion used in convert_snapshot_to_order
        let void_type = VoidType::LossSettled;
        let result: Option<String> = Some(&void_type).map(|v| {
            serde_json::to_value(v)
                .ok()
                .and_then(|val| val.as_str().map(String::from))
                .unwrap_or_default()
        });
        assert_eq!(result, Some("LOSS_SETTLED".to_string()));

        let void_type2 = VoidType::Cancelled;
        let result2: Option<String> = Some(&void_type2).map(|v| {
            serde_json::to_value(v)
                .ok()
                .and_then(|val| val.as_str().map(String::from))
                .unwrap_or_default()
        });
        assert_eq!(result2, Some("CANCELLED".to_string()));

        let loss_reason = LossReason::CustomerFled;
        let result3: Option<String> = Some(&loss_reason).map(|r| {
            serde_json::to_value(r)
                .ok()
                .and_then(|val| val.as_str().map(String::from))
                .unwrap_or_default()
        });
        assert_eq!(result3, Some("CUSTOMER_FLED".to_string()));
    }
}
