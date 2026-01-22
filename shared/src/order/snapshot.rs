//! Order snapshot - computed state from event stream
//!
//! The snapshot includes a `state_checksum` field for drift detection.
//! Clients can compare their locally computed checksum with the server's
//! to detect if the reducer logic has diverged.

use super::types::{CartItemSnapshot, PaymentRecord};
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
    /// Order status
    pub status: OrderStatus,
    /// Items in the order
    pub items: Vec<CartItemSnapshot>,
    /// Payment records
    pub payments: Vec<PaymentRecord>,
    /// Subtotal before surcharge
    pub subtotal: f64,
    /// Tax amount
    #[serde(default)]
    pub tax: f64,
    /// Discount amount
    #[serde(default)]
    pub discount: f64,
    /// Total amount
    pub total: f64,
    /// Amount paid
    #[serde(default)]
    pub paid_amount: f64,
    /// Quantities paid per item (for split bill)
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub paid_item_quantities: std::collections::HashMap<String, i32>,
    /// Receipt number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_number: Option<String>,
    /// Whether this is a pre-payment order
    #[serde(default)]
    pub is_pre_payment: bool,
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
        let now = chrono::Utc::now().timestamp_millis();
        let mut snapshot = Self {
            order_id,
            table_id: None,
            table_name: None,
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
            status: OrderStatus::Active,
            items: Vec::new(),
            payments: Vec::new(),
            subtotal: 0.0,
            tax: 0.0,
            discount: 0.0,
            total: 0.0,
            paid_amount: 0.0,
            paid_item_quantities: std::collections::HashMap::new(),
            receipt_number: None,
            is_pre_payment: false,
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
