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
/// 包含摘要层（永久保存，含 VeriFactu desglose）和详情层（30 天滚动）。
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
    pub created_at: i64,
    /// VeriFactu 税率分拆
    pub desglose: Vec<TaxDesglose>,

    // ── 详情层（30 天滚动） ──
    pub detail: OrderDetailPayload,
}

/// VeriFactu 税率分拆
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_info: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
