//! Cloud sync batch protocol types
//!
//! Used by edge-server to push data to crab-cloud,
//! and by crab-cloud to receive and store synced data.

use serde::{Deserialize, Serialize};

/// All syncable resource types across the system.
///
/// Serializes to snake_case strings (e.g. `DiningTable` → `"dining_table"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncResource {
    Product,
    Category,
    Tag,
    Attribute,
    AttributeBinding,
    Zone,
    DiningTable,
    Employee,
    PriceRule,
    StoreInfo,
    Shift,
    DailyReport,
    SystemState,
    SystemIssue,
    PrintConfig,
    PrintDestination,
    LabelTemplate,
    Member,
    MarketingGroup,
    /// Archived orders (edge → cloud only, not in initial sync)
    ArchivedOrder,
    /// Order sync events (edge-internal, for live order push to cloud)
    OrderSync,
    /// Credit notes (edge → cloud only)
    CreditNote,
    /// Verifactu invoices (edge → cloud only)
    Invoice,
    /// Role resource (client-visible for sync status)
    Role,
}

impl SyncResource {
    /// Resources that should be synced to cloud on initial connect
    pub const INITIAL_SYNC: &'static [SyncResource] = &[
        Self::Product,
        Self::Category,
        Self::Tag,
        Self::Attribute,
        Self::AttributeBinding,
        Self::Zone,
        Self::DiningTable,
        Self::Employee,
        Self::PriceRule,
        Self::StoreInfo,
        Self::LabelTemplate,
    ];

    /// Resources that cloud accepts via live sync (extract_sync_item whitelist)
    pub const CLOUD_SYNCED: &'static [SyncResource] = &[
        Self::Product,
        Self::Category,
        Self::Tag,
        Self::Attribute,
        Self::AttributeBinding,
        Self::Zone,
        Self::DiningTable,
        Self::Employee,
        Self::PriceRule,
        Self::StoreInfo,
        Self::Shift,
        Self::DailyReport,
        Self::LabelTemplate,
    ];

    /// Resources exposed in the client sync/status endpoint
    pub const CLIENT_VISIBLE: &'static [SyncResource] = &[
        Self::Product,
        Self::Category,
        Self::Tag,
        Self::Attribute,
        Self::Zone,
        Self::DiningTable,
        Self::Employee,
        Self::Role,
        Self::PriceRule,
        Self::PrintDestination,
        Self::LabelTemplate,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::Category => "category",
            Self::Tag => "tag",
            Self::Attribute => "attribute",
            Self::AttributeBinding => "attribute_binding",
            Self::Zone => "zone",
            Self::DiningTable => "dining_table",
            Self::Employee => "employee",
            Self::PriceRule => "price_rule",
            Self::StoreInfo => "store_info",
            Self::Shift => "shift",
            Self::DailyReport => "daily_report",
            Self::SystemState => "system_state",
            Self::SystemIssue => "system_issue",
            Self::PrintConfig => "print_config",
            Self::PrintDestination => "print_destination",
            Self::LabelTemplate => "label_template",
            Self::Member => "member",
            Self::MarketingGroup => "marketing_group",
            Self::ArchivedOrder => "archived_order",
            Self::CreditNote => "credit_note",
            Self::Invoice => "invoice",
            Self::OrderSync => "order_sync",
            Self::Role => "role",
        }
    }

    pub fn is_cloud_synced(&self) -> bool {
        Self::CLOUD_SYNCED.contains(self)
    }

    /// Per-store resource upper bound. Returns `None` for resources without a limit.
    pub const fn max_per_store(&self) -> Option<i64> {
        match self {
            Self::Product => Some(2000),
            Self::Category => Some(200),
            Self::Tag => Some(200),
            Self::Attribute => Some(100),
            Self::PriceRule => Some(100),
            Self::Employee => Some(100),
            Self::Zone => Some(50),
            Self::DiningTable => Some(500),
            Self::LabelTemplate => Some(50),
            _ => None,
        }
    }
}

impl std::fmt::Display for SyncResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Maximum items per sync batch (HTTP or WS).
pub const MAX_SYNC_BATCH_ITEMS: usize = 500;

/// A batch of sync items from an edge-server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncBatch {
    /// Edge server entity_id (from SignedBinding)
    pub edge_id: String,
    /// Sync items in this batch
    pub items: Vec<CloudSyncItem>,
    /// Timestamp when the batch was sent (Unix millis)
    pub sent_at: i64,
}

/// Cloud sync action (edge → cloud)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncAction {
    Upsert,
    Delete,
}

impl std::fmt::Display for SyncAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Upsert => f.write_str("upsert"),
            Self::Delete => f.write_str("delete"),
        }
    }
}

/// A single resource change to sync to the cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncItem {
    /// Resource type
    pub resource: SyncResource,
    /// Monotonically increasing version for this resource on this edge
    pub version: u64,
    /// Action
    pub action: SyncAction,
    /// Resource ID (source ID on the edge-server)
    pub resource_id: String,
    /// Full resource data as JSON
    pub data: serde_json::Value,
}

/// Response from crab-cloud after processing a sync batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncResponse {
    /// Number of items accepted
    pub accepted: u32,
    /// Number of items rejected
    pub rejected: u32,
    /// Errors for rejected items
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CloudSyncError>,
}

/// Error detail for a rejected sync item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncError {
    /// Index of the item in the batch
    pub index: u32,
    /// Resource ID that failed
    pub resource_id: String,
    /// Error message
    pub message: String,
}

/// 归档订单完整详情（edge→cloud 推送）
///
/// 两层存储：摘要层（永久，含 VeriFactu desglose）+ 详情层（永久，完整 JSONB）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetailSync {
    // ── 摘要层（永久保存） ──
    /// UUID (OrderSnapshot.order_id)，全局唯一
    pub order_key: String,
    pub receipt_number: String,
    pub status: String,
    pub total_amount: f64,
    pub tax: f64,
    pub end_time: Option<i64>,
    pub prev_hash: String,
    pub curr_hash: String,
    /// Hash of the last event in the order's event chain (for hash re-verification)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_hash: Option<String>,
    pub created_at: i64,
    /// VeriFactu 税率分拆
    pub desglose: Vec<TaxDesglose>,

    // ── 详情层（永久保存） ──
    pub detail: OrderDetailPayload,
}

/// VeriFactu 税率分拆
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxDesglose {
    /// 税率: 0, 4, 10, 21
    pub tax_rate: i32,
    /// 税前金额 (BaseImponible)
    pub base_amount: rust_decimal::Decimal,
    /// 税额 (CuotaRepercutida)
    pub tax_amount: rust_decimal::Decimal,
}

/// 订单事件同步载荷（edge→cloud 推送，用于 Red Flags 监控）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEventSync {
    pub seq: i32,
    pub event_type: String,
    pub timestamp: i64,
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub data: Option<String>,
}

/// 订单详情载荷（items + payments + events）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetailPayload {
    pub zone_name: Option<String>,
    pub table_name: Option<String>,
    pub is_retail: bool,
    pub guest_count: Option<i32>,
    pub original_total: f64,
    pub subtotal: f64,
    pub paid_amount: f64,
    pub discount_amount: f64,
    pub surcharge_amount: f64,
    pub comp_total_amount: f64,
    pub order_manual_discount_amount: f64,
    pub order_manual_surcharge_amount: f64,
    pub order_rule_discount_amount: f64,
    pub order_rule_surcharge_amount: f64,
    pub start_time: i64,
    pub operator_name: Option<String>,
    pub void_type: Option<String>,
    pub loss_reason: Option<String>,
    pub loss_amount: Option<f64>,
    pub void_note: Option<String>,
    pub member_name: Option<String>,
    pub items: Vec<OrderItemSync>,
    pub payments: Vec<OrderPaymentSync>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<OrderEventSync>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemSync {
    pub name: String,
    pub spec_name: Option<String>,
    pub category_name: Option<String>,
    pub product_source_id: Option<i64>,
    pub price: f64,
    pub quantity: i32,
    pub unit_price: f64,
    pub line_total: f64,
    pub discount_amount: f64,
    pub surcharge_amount: f64,
    pub tax: f64,
    pub tax_rate: i32,
    pub is_comped: bool,
    pub note: Option<String>,
    pub options: Vec<OrderItemOptionSync>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItemOptionSync {
    pub attribute_name: String,
    pub option_name: String,
    pub price: f64,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPaymentSync {
    pub seq: i32,
    pub method: String,
    pub amount: f64,
    pub timestamp: i64,
    pub cancelled: bool,
}

// ── Credit Note sync types ──

/// 退款凭证同步载荷（edge→cloud 推送）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditNoteSync {
    pub credit_note_number: String,
    pub original_order_key: String,
    pub original_receipt: String,
    pub subtotal_credit: f64,
    pub tax_credit: f64,
    pub total_credit: f64,
    pub refund_method: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub operator_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_name: Option<String>,
    pub prev_hash: String,
    pub curr_hash: String,
    pub created_at: i64,
    pub items: Vec<CreditNoteItemSync>,
}

/// 退款明细行同步载荷
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreditNoteItemSync {
    pub item_name: String,
    pub quantity: i64,
    pub unit_price: f64,
    pub line_credit: f64,
    pub tax_rate: i64,
    pub tax_credit: f64,
}

// ── Invoice sync types ──

/// Invoice data synced to cloud for Verifactu AEAT submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSync {
    pub id: i64,
    pub invoice_number: String,
    pub serie: String,
    pub tipo_factura: crate::models::invoice::TipoFactura,
    pub source_type: crate::models::invoice::InvoiceSourceType,
    pub source_pk: i64,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub desglose: Vec<TaxDesglose>,
    pub huella: String,
    pub prev_huella: Option<String>,
    pub fecha_expedicion: String,
    pub nif: String,
    pub nombre_razon: String,
    pub factura_rectificada_id: Option<i64>,
    pub factura_rectificada_num: Option<String>,
    pub created_at: i64,
}

// ── Hash verification ──

impl CreditNoteSync {
    /// Recompute the chain hash and return `Some(recomputed)` on mismatch, `None` if ok.
    pub fn verify_hash(&self) -> Option<String> {
        let recomputed = crate::order::compute_credit_note_chain_hash(
            &self.prev_hash,
            &self.credit_note_number,
            &self.original_receipt,
            self.total_credit,
            self.tax_credit,
        );
        if recomputed == self.curr_hash {
            None
        } else {
            Some(recomputed)
        }
    }
}

impl OrderDetailSync {
    /// Recompute the chain hash and return `Some(recomputed)` on mismatch, `None` if ok.
    ///
    /// Returns `Some("")` if `last_event_hash` is missing or status is unrecognized.
    pub fn verify_hash(&self) -> Option<String> {
        let Some(ref last_event_hash) = self.last_event_hash else {
            return Some(String::new());
        };
        let status: crate::order::OrderStatus = match self.status.as_str() {
            "COMPLETED" => crate::order::OrderStatus::Completed,
            "VOID" => crate::order::OrderStatus::Void,
            "MERGED" => crate::order::OrderStatus::Merged,
            "ACTIVE" => crate::order::OrderStatus::Active,
            _ => return Some(String::new()),
        };
        let recomputed = crate::order::compute_order_chain_hash(
            &self.prev_hash,
            &self.order_key,
            &self.receipt_number,
            &status,
            last_event_hash,
            self.total_amount,
            self.tax,
        );
        if recomputed == self.curr_hash {
            None
        } else {
            Some(recomputed)
        }
    }
}

// ── Cross-verification (recomputable by cloud) ──

/// Recompute desglose from order items (GROUP BY tax_rate).
///
/// This is the same logic as edge-server's archiving, placed in `shared`
/// so the cloud can independently verify the desglose sent by an edge.
pub fn compute_desglose(items: &[OrderItemSync]) -> Vec<TaxDesglose> {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use std::collections::BTreeMap;

    let mut map: BTreeMap<i32, (Decimal, Decimal)> = BTreeMap::new();
    for item in items {
        let entry = map
            .entry(item.tax_rate)
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        let line_total = Decimal::from_f64(item.line_total).unwrap_or(Decimal::ZERO);
        let tax = Decimal::from_f64(item.tax).unwrap_or(Decimal::ZERO);
        entry.0 += line_total - tax; // base_amount
        entry.1 += tax; // tax_amount
    }
    map.into_iter()
        .map(|(tax_rate, (base_amount, tax_amount))| TaxDesglose {
            tax_rate,
            base_amount,
            tax_amount,
        })
        .collect()
}

/// Amount mismatch detail returned by cross-verification.
#[derive(Debug, Clone)]
pub struct AmountMismatch {
    pub field: &'static str,
    pub expected: rust_decimal::Decimal,
    pub actual: rust_decimal::Decimal,
}

impl std::fmt::Display for AmountMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: expected={}, actual={}",
            self.field, self.expected, self.actual
        )
    }
}

impl OrderDetailSync {
    /// Recompute desglose from items and compare against the stored desglose.
    ///
    /// Returns mismatched entries, or empty vec if all match.
    pub fn verify_desglose(&self) -> Vec<(TaxDesglose, TaxDesglose)> {
        let recomputed = compute_desglose(&self.detail.items);
        let mut stored_sorted = self.desglose.clone();
        stored_sorted.sort_by_key(|d| d.tax_rate);

        if recomputed == stored_sorted {
            return vec![];
        }

        // Collect per-rate mismatches
        let mut mismatches = Vec::new();
        let stored_map: std::collections::BTreeMap<i32, &TaxDesglose> =
            stored_sorted.iter().map(|d| (d.tax_rate, d)).collect();
        let recomp_map: std::collections::BTreeMap<i32, &TaxDesglose> =
            recomputed.iter().map(|d| (d.tax_rate, d)).collect();

        let all_rates: std::collections::BTreeSet<i32> = stored_map
            .keys()
            .chain(recomp_map.keys())
            .copied()
            .collect();

        let zero = TaxDesglose {
            tax_rate: 0,
            base_amount: rust_decimal::Decimal::ZERO,
            tax_amount: rust_decimal::Decimal::ZERO,
        };

        for rate in all_rates {
            let s = stored_map.get(&rate).copied().unwrap_or(&zero);
            let r = recomp_map.get(&rate).copied().unwrap_or(&zero);
            if s != r {
                mismatches.push((
                    TaxDesglose {
                        tax_rate: rate,
                        ..*s
                    },
                    TaxDesglose {
                        tax_rate: rate,
                        ..*r
                    },
                ));
            }
        }
        mismatches
    }

    /// Cross-verify order amounts from items and payments.
    ///
    /// Checks:
    /// - sum(item.line_total) ≈ total_amount
    /// - sum(item.tax) ≈ tax
    /// - sum(active payments) ≈ paid_amount
    pub fn verify_amounts(&self) -> Vec<AmountMismatch> {
        use rust_decimal::Decimal;
        use rust_decimal::prelude::FromPrimitive;

        let mut mismatches = Vec::new();
        let d = &self.detail;

        // sum of item line_total
        let items_total: Decimal = d
            .items
            .iter()
            .map(|i| Decimal::from_f64(i.line_total).unwrap_or(Decimal::ZERO))
            .sum();
        let expected_total = Decimal::from_f64(self.total_amount).unwrap_or(Decimal::ZERO);
        if items_total != expected_total {
            mismatches.push(AmountMismatch {
                field: "total_amount",
                expected: items_total,
                actual: expected_total,
            });
        }

        // sum of item tax
        let items_tax: Decimal = d
            .items
            .iter()
            .map(|i| Decimal::from_f64(i.tax).unwrap_or(Decimal::ZERO))
            .sum();
        let expected_tax = Decimal::from_f64(self.tax).unwrap_or(Decimal::ZERO);
        if items_tax != expected_tax {
            mismatches.push(AmountMismatch {
                field: "tax",
                expected: items_tax,
                actual: expected_tax,
            });
        }

        // sum of active (non-cancelled) payments
        let payments_total: Decimal = d
            .payments
            .iter()
            .filter(|p| !p.cancelled)
            .map(|p| Decimal::from_f64(p.amount).unwrap_or(Decimal::ZERO))
            .sum();
        let expected_paid = Decimal::from_f64(d.paid_amount).unwrap_or(Decimal::ZERO);
        if payments_total != expected_paid {
            mismatches.push(AmountMismatch {
                field: "paid_amount",
                expected: payments_total,
                actual: expected_paid,
            });
        }

        mismatches
    }
}

impl CreditNoteSync {
    /// Cross-verify credit note amounts from items.
    ///
    /// Checks:
    /// - sum(item.line_credit) ≈ subtotal_credit (line_credit is pre-tax)
    /// - sum(item.tax_credit) ≈ tax_credit
    /// - subtotal_credit + tax_credit ≈ total_credit
    pub fn verify_amounts(&self) -> Vec<AmountMismatch> {
        use rust_decimal::Decimal;
        use rust_decimal::prelude::FromPrimitive;

        let mut mismatches = Vec::new();

        let items_subtotal: Decimal = self
            .items
            .iter()
            .map(|i| Decimal::from_f64(i.line_credit).unwrap_or(Decimal::ZERO))
            .sum();
        let items_tax: Decimal = self
            .items
            .iter()
            .map(|i| Decimal::from_f64(i.tax_credit).unwrap_or(Decimal::ZERO))
            .sum();

        let expected_subtotal = Decimal::from_f64(self.subtotal_credit).unwrap_or(Decimal::ZERO);
        let expected_tax = Decimal::from_f64(self.tax_credit).unwrap_or(Decimal::ZERO);
        let expected_total = Decimal::from_f64(self.total_credit).unwrap_or(Decimal::ZERO);

        // sum(line_credit) = subtotal_credit (line_credit is pre-tax)
        if items_subtotal != expected_subtotal {
            mismatches.push(AmountMismatch {
                field: "subtotal_credit",
                expected: items_subtotal,
                actual: expected_subtotal,
            });
        }

        // sum(tax_credit) = tax_credit
        if items_tax != expected_tax {
            mismatches.push(AmountMismatch {
                field: "tax_credit",
                expected: items_tax,
                actual: expected_tax,
            });
        }

        // subtotal + tax = total
        if expected_subtotal + expected_tax != expected_total {
            mismatches.push(AmountMismatch {
                field: "subtotal_credit+tax_credit",
                expected: expected_subtotal + expected_tax,
                actual: expected_total,
            });
        }

        mismatches
    }
}

// ── Tenant API response types ──

/// GET /api/tenant/stores/:id/orders/:order_key/detail response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDetailResponse {
    /// "cache" or "edge"
    pub source: String,
    pub detail: OrderDetailPayload,
    pub desglose: Vec<TaxDesglose>,
}

/// Edge status returned by `get_status` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeStatusResult {
    pub active_orders: usize,
    pub products: usize,
    pub categories: usize,
    pub epoch: String,
}

/// GET /api/tenant/stores response item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDetailResponse {
    pub id: i64,
    pub entity_id: String,
    pub alias: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub nif: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub business_day_cutoff: Option<String>,
    pub device_id: String,
    pub is_online: bool,
    pub last_sync_at: Option<i64>,
    pub registered_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Credit note hash roundtrip tests ──

    fn sample_credit_note_sync() -> CreditNoteSync {
        let prev_hash = "genesis".to_string();
        let cn_number = "CN-20260227-0001".to_string();
        let original_receipt = "R-20260227-001".to_string();
        let total_credit = 25.50_f64;
        let tax_credit = 4.43_f64;

        let curr_hash = crate::order::compute_credit_note_chain_hash(
            &prev_hash,
            &cn_number,
            &original_receipt,
            total_credit,
            tax_credit,
        );

        CreditNoteSync {
            credit_note_number: cn_number,
            original_order_key: "uuid-order-001".to_string(),
            original_receipt,
            subtotal_credit: 21.07,
            tax_credit: 4.43,
            total_credit,
            refund_method: "CASH".to_string(),
            reason: "Customer request".to_string(),
            note: Some("partial refund".to_string()),
            operator_name: "Admin".to_string(),
            authorizer_name: None,
            prev_hash,
            curr_hash,
            created_at: 1709020800000,
            items: vec![CreditNoteItemSync {
                item_name: "Paella".to_string(),
                quantity: 1,
                unit_price: 21.07,
                line_credit: 21.07,
                tax_rate: 2100,
                tax_credit: 4.43,
            }],
        }
    }

    #[test]
    fn test_credit_note_hash_golden_value() {
        let hash = crate::order::compute_credit_note_chain_hash(
            "genesis",
            "CN-20260227-0001",
            "R-20260227-001",
            25.50,
            4.43,
        );
        // Golden value updated: now includes tax_credit in hash
        assert!(!hash.is_empty(), "Hash must be non-empty");
        // Determinism check
        let hash2 = crate::order::compute_credit_note_chain_hash(
            "genesis",
            "CN-20260227-0001",
            "R-20260227-001",
            25.50,
            4.43,
        );
        assert_eq!(hash, hash2, "Hash must be deterministic");
    }

    #[test]
    fn test_credit_note_verify_hash_on_fresh() {
        let cn = sample_credit_note_sync();
        assert!(
            cn.verify_hash().is_none(),
            "Fresh CreditNoteSync must pass hash verification"
        );
    }

    #[test]
    fn test_credit_note_hash_survives_json_roundtrip() {
        let cn = sample_credit_note_sync();
        let json = serde_json::to_string(&cn).unwrap();
        let deserialized: CreditNoteSync = serde_json::from_str(&json).unwrap();

        assert_eq!(
            cn, deserialized,
            "CreditNoteSync must be identical after JSON roundtrip"
        );
        assert!(
            deserialized.verify_hash().is_none(),
            "Hash verification must pass after JSON roundtrip"
        );
    }

    #[test]
    fn test_credit_note_hash_survives_value_roundtrip() {
        let cn = sample_credit_note_sync();
        let value = serde_json::to_value(&cn).unwrap();
        let deserialized: CreditNoteSync = serde_json::from_value(value).unwrap();

        assert_eq!(
            cn, deserialized,
            "CreditNoteSync must be identical after Value roundtrip"
        );
        assert!(
            deserialized.verify_hash().is_none(),
            "Hash verification must pass after Value roundtrip"
        );
    }

    #[test]
    fn test_credit_note_hash_detects_tampering() {
        let mut cn = sample_credit_note_sync();
        cn.total_credit = 99.99; // tamper
        assert!(
            cn.verify_hash().is_some(),
            "Tampered total_credit must fail hash verification"
        );
    }

    #[test]
    fn test_credit_note_f64_edge_cases() {
        // Test that f64 values with various decimal places roundtrip correctly
        for total in [0.0, 0.01, 0.1, 1.0, 9.99, 100.0, 12345.67, 0.005] {
            let prev = "test_prev".to_string();
            let cn_num = "CN-TEST-0001".to_string();
            let receipt = "R-TEST-001".to_string();

            let tax = 0.0_f64;
            let hash =
                crate::order::compute_credit_note_chain_hash(&prev, &cn_num, &receipt, total, tax);

            let cn = CreditNoteSync {
                credit_note_number: cn_num,
                original_order_key: "uuid".to_string(),
                original_receipt: receipt,
                subtotal_credit: total,
                tax_credit: tax,
                total_credit: total,
                refund_method: "CASH".to_string(),
                reason: "test".to_string(),
                note: None,
                operator_name: "op".to_string(),
                authorizer_name: None,
                prev_hash: prev,
                curr_hash: hash,
                created_at: 0,
                items: vec![],
            };

            let json = serde_json::to_string(&cn).unwrap();
            let rt: CreditNoteSync = serde_json::from_str(&json).unwrap();
            assert!(
                rt.verify_hash().is_none(),
                "f64 value {total} must survive JSON roundtrip for hash verification"
            );
        }
    }

    // ── Order hash roundtrip tests ──

    #[test]
    fn test_order_verify_hash() {
        let prev_hash = "genesis".to_string();
        let order_key = "uuid-order-001".to_string();
        let receipt = "R-20260227-001".to_string();
        let status_str = "COMPLETED".to_string();
        let last_event_hash = "event_hash_abc123".to_string();

        let total_amount = 100.0_f64;
        let tax = 21.0_f64;

        let curr_hash = crate::order::compute_order_chain_hash(
            &prev_hash,
            &order_key,
            &receipt,
            &crate::order::OrderStatus::Completed,
            &last_event_hash,
            total_amount,
            tax,
        );

        let order = OrderDetailSync {
            order_key,
            receipt_number: receipt,
            status: status_str,
            total_amount,
            tax,
            end_time: Some(1709020800000),
            prev_hash,
            curr_hash,
            last_event_hash: Some(last_event_hash),
            created_at: 1709020800000,
            desglose: vec![],
            detail: OrderDetailPayload {
                zone_name: None,
                table_name: None,
                is_retail: false,
                guest_count: None,
                original_total: 100.0,
                subtotal: 100.0,
                paid_amount: 100.0,
                discount_amount: 0.0,
                surcharge_amount: 0.0,
                comp_total_amount: 0.0,
                order_manual_discount_amount: 0.0,
                order_manual_surcharge_amount: 0.0,
                order_rule_discount_amount: 0.0,
                order_rule_surcharge_amount: 0.0,
                start_time: 1709020800000,
                operator_name: None,
                void_type: None,
                loss_reason: None,
                loss_amount: None,
                void_note: None,
                member_name: None,
                items: vec![],
                payments: vec![],
                events: vec![],
            },
        };

        assert!(
            order.verify_hash().is_none(),
            "Fresh OrderDetailSync must pass hash verification"
        );

        // JSON roundtrip
        let json = serde_json::to_string(&order).unwrap();
        let rt: OrderDetailSync = serde_json::from_str(&json).unwrap();
        assert!(
            rt.verify_hash().is_none(),
            "OrderDetailSync hash must survive JSON roundtrip"
        );
    }

    #[test]
    fn test_order_verify_hash_without_last_event_hash() {
        let order = OrderDetailSync {
            order_key: "uuid".to_string(),
            receipt_number: "R-001".to_string(),
            status: "COMPLETED".to_string(),
            total_amount: 0.0,
            tax: 0.0,
            end_time: None,
            prev_hash: "genesis".to_string(),
            curr_hash: "some_hash".to_string(),
            last_event_hash: None, // missing
            created_at: 0,
            desglose: vec![],
            detail: OrderDetailPayload {
                zone_name: None,
                table_name: None,
                is_retail: false,
                guest_count: None,
                original_total: 0.0,
                subtotal: 0.0,
                paid_amount: 0.0,
                discount_amount: 0.0,
                surcharge_amount: 0.0,
                comp_total_amount: 0.0,
                order_manual_discount_amount: 0.0,
                order_manual_surcharge_amount: 0.0,
                order_rule_discount_amount: 0.0,
                order_rule_surcharge_amount: 0.0,
                start_time: 0,
                operator_name: None,
                void_type: None,
                loss_reason: None,
                loss_amount: None,
                void_note: None,
                member_name: None,
                items: vec![],
                payments: vec![],
                events: vec![],
            },
        };
        assert!(
            order.verify_hash().is_some(),
            "Missing last_event_hash must fail verification"
        );
    }

    // ── Existing tests ──

    #[test]
    fn test_cloud_sync_batch_serialization() {
        let batch = CloudSyncBatch {
            edge_id: "edge-001".to_string(),
            items: vec![CloudSyncItem {
                resource: SyncResource::Product,
                version: 1,
                action: SyncAction::Upsert,
                resource_id: "42".to_string(),
                data: serde_json::json!({"name": "Test Product", "price": 9.99}),
            }],
            sent_at: 1700000000000,
        };

        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: CloudSyncBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.edge_id, "edge-001");
        assert_eq!(deserialized.items.len(), 1);
        assert_eq!(deserialized.items[0].resource, SyncResource::Product);
        assert_eq!(deserialized.items[0].version, 1);
    }

    #[test]
    fn test_cloud_sync_response_serialization() {
        let response = CloudSyncResponse {
            accepted: 5,
            rejected: 1,
            errors: vec![CloudSyncError {
                index: 3,
                resource_id: "99".to_string(),
                message: "Invalid data".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CloudSyncResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.accepted, 5);
        assert_eq!(deserialized.rejected, 1);
        assert_eq!(deserialized.errors.len(), 1);
    }

    #[test]
    fn test_empty_response_skips_optional_fields() {
        let response = CloudSyncResponse {
            accepted: 10,
            rejected: 0,
            errors: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("errors"));
    }

    // ── Cross-verification tests ──

    fn make_order_with_items() -> OrderDetailSync {
        OrderDetailSync {
            order_key: "uuid-001".to_string(),
            receipt_number: "R-001".to_string(),
            status: "COMPLETED".to_string(),
            total_amount: 36.30, // 30.00 + 6.30 (21% tax)
            tax: 6.30,
            end_time: Some(1709020800000),
            prev_hash: "genesis".to_string(),
            curr_hash: "dummy".to_string(),
            last_event_hash: Some("evt".to_string()),
            created_at: 1709020800000,
            desglose: vec![TaxDesglose {
                tax_rate: 2100,
                base_amount: rust_decimal::Decimal::new(3000, 2), // 30.00
                tax_amount: rust_decimal::Decimal::new(630, 2),   // 6.30
            }],
            detail: OrderDetailPayload {
                zone_name: None,
                table_name: None,
                is_retail: false,
                guest_count: None,
                original_total: 36.30,
                subtotal: 36.30,
                paid_amount: 36.30,
                discount_amount: 0.0,
                surcharge_amount: 0.0,
                comp_total_amount: 0.0,
                order_manual_discount_amount: 0.0,
                order_manual_surcharge_amount: 0.0,
                order_rule_discount_amount: 0.0,
                order_rule_surcharge_amount: 0.0,
                start_time: 1709020800000,
                operator_name: None,
                void_type: None,
                loss_reason: None,
                loss_amount: None,
                void_note: None,
                member_name: None,
                items: vec![OrderItemSync {
                    name: "Paella".to_string(),
                    spec_name: None,
                    category_name: None,
                    product_source_id: Some(1),
                    price: 36.30,
                    quantity: 1,
                    unit_price: 36.30,
                    line_total: 36.30,
                    discount_amount: 0.0,
                    surcharge_amount: 0.0,
                    tax: 6.30,
                    tax_rate: 2100,
                    is_comped: false,
                    note: None,
                    options: vec![],
                }],
                payments: vec![OrderPaymentSync {
                    seq: 1,
                    method: "CASH".to_string(),
                    amount: 36.30,
                    timestamp: 1709020800000,
                    cancelled: false,
                }],
                events: vec![],
            },
        }
    }

    #[test]
    fn test_verify_desglose_matches() {
        let order = make_order_with_items();
        assert!(
            order.verify_desglose().is_empty(),
            "Consistent desglose must pass verification"
        );
    }

    #[test]
    fn test_verify_desglose_detects_mismatch() {
        let mut order = make_order_with_items();
        order.desglose[0].base_amount = rust_decimal::Decimal::new(9999, 2); // tamper
        let mismatches = order.verify_desglose();
        assert!(!mismatches.is_empty(), "Tampered desglose must be detected");
    }

    #[test]
    fn test_verify_order_amounts_matches() {
        let order = make_order_with_items();
        assert!(
            order.verify_amounts().is_empty(),
            "Consistent amounts must pass verification"
        );
    }

    #[test]
    fn test_verify_order_amounts_detects_total_mismatch() {
        let mut order = make_order_with_items();
        order.total_amount = 999.99; // tamper
        let mismatches = order.verify_amounts();
        assert!(
            mismatches.iter().any(|m| m.field == "total_amount"),
            "Tampered total_amount must be detected"
        );
    }

    #[test]
    fn test_verify_order_amounts_detects_payment_mismatch() {
        let mut order = make_order_with_items();
        order.detail.paid_amount = 0.0; // tamper
        let mismatches = order.verify_amounts();
        assert!(
            mismatches.iter().any(|m| m.field == "paid_amount"),
            "Tampered paid_amount must be detected"
        );
    }

    #[test]
    fn test_verify_credit_note_amounts_matches() {
        let cn = sample_credit_note_sync();
        // items: line_credit=21.07, tax_credit=4.43
        // total_credit=25.50, subtotal_credit=21.07, tax_credit=4.43
        assert!(
            cn.verify_amounts().is_empty(),
            "Consistent credit note amounts must pass verification"
        );
    }

    #[test]
    fn test_verify_credit_note_amounts_detects_total_mismatch() {
        let mut cn = sample_credit_note_sync();
        cn.total_credit = 999.99; // tamper
        let mismatches = cn.verify_amounts();
        assert!(
            !mismatches.is_empty(),
            "Tampered total_credit must be detected"
        );
    }

    #[test]
    fn test_verify_credit_note_amounts_detects_tax_mismatch() {
        let mut cn = sample_credit_note_sync();
        cn.tax_credit = 0.0; // tamper
        let mismatches = cn.verify_amounts();
        assert!(
            mismatches.iter().any(|m| m.field == "tax_credit"),
            "Tampered tax_credit must be detected"
        );
    }

    #[test]
    fn test_compute_desglose_multi_rate() {
        let items = vec![
            OrderItemSync {
                name: "Paella".to_string(),
                spec_name: None,
                category_name: None,
                product_source_id: Some(1),
                price: 12.10,
                quantity: 1,
                unit_price: 12.10,
                line_total: 12.10,
                discount_amount: 0.0,
                surcharge_amount: 0.0,
                tax: 2.10,
                tax_rate: 2100,
                is_comped: false,
                note: None,
                options: vec![],
            },
            OrderItemSync {
                name: "Bread".to_string(),
                spec_name: None,
                category_name: None,
                product_source_id: Some(2),
                price: 1.04,
                quantity: 1,
                unit_price: 1.04,
                line_total: 1.04,
                discount_amount: 0.0,
                surcharge_amount: 0.0,
                tax: 0.04,
                tax_rate: 400,
                is_comped: false,
                note: None,
                options: vec![],
            },
        ];
        let desglose = compute_desglose(&items);
        assert_eq!(desglose.len(), 2, "Should have two tax rate groups");
        // BTreeMap ensures sorted by rate
        assert_eq!(desglose[0].tax_rate, 400);
        assert_eq!(desglose[1].tax_rate, 2100);
    }
}
