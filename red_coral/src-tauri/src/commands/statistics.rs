//! Statistics Commands
//!
//! 数据统计相关的 Tauri 命令

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use tracing::warn;

use crate::core::response::ApiResponse;
use crate::core::ClientBridge;

// ============================================================================
// Response Types — aligned with edge-server StoreOverview
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreOverview {
    pub revenue: f64,
    pub net_revenue: f64,
    pub orders: i32,
    pub guests: i32,
    pub average_order_value: f64,
    pub per_guest_spend: f64,
    pub average_dining_minutes: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    pub avg_items_per_order: f64,
    pub voided_orders: i32,
    pub voided_amount: f64,
    pub loss_orders: i32,
    pub loss_amount: f64,
    pub anulacion_count: i32,
    pub anulacion_amount: f64,
    pub refund_count: i32,
    pub refund_amount: f64,
    pub revenue_trend: Vec<RevenueTrendPoint>,
    pub daily_trend: Vec<DailyTrendPoint>,
    pub payment_breakdown: Vec<PaymentBreakdownEntry>,
    pub tax_breakdown: Vec<TaxBreakdownEntry>,
    pub category_sales: Vec<CategorySaleEntry>,
    pub top_products: Vec<TopProductEntry>,
    pub tag_sales: Vec<TagSaleEntry>,
    pub refund_method_breakdown: Vec<RefundMethodEntry>,
    pub service_type_breakdown: Vec<ServiceTypeEntry>,
    pub zone_sales: Vec<ZoneSaleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueTrendPoint {
    pub hour: i32,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTrendPoint {
    pub date: String,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentBreakdownEntry {
    pub method: String,
    pub amount: f64,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxBreakdownEntry {
    pub tax_rate: f64,
    pub base_amount: f64,
    pub tax_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySaleEntry {
    pub name: String,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProductEntry {
    pub name: String,
    pub quantity: i32,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagSaleEntry {
    pub name: String,
    pub color: Option<String>,
    pub revenue: f64,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundMethodEntry {
    pub method: String,
    pub amount: f64,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTypeEntry {
    pub service_type: String,
    pub revenue: f64,
    pub orders: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneSaleEntry {
    pub zone_name: String,
    pub revenue: f64,
    pub orders: i32,
    pub guests: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesReportItem {
    pub order_id: String,
    pub receipt_number: Option<String>,
    pub date: String,
    pub total: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesReportResponse {
    pub items: Vec<SalesReportItem>,
    pub total: i32,
    pub page: i32,
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
}

// ============================================================================
// Commands
// ============================================================================

/// 获取统计数据 — supports both legacy timeRange and from/to millis
#[tauri::command]
pub async fn get_statistics(
    bridge: State<'_, Arc<ClientBridge>>,
    time_range: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
) -> Result<ApiResponse<StoreOverview>, String> {
    let mut path = String::from("/api/statistics?");
    let mut first = true;
    let mut add = |key: &str, val: &str| {
        if !first {
            path.push('&');
        }
        path.push_str(&format!("{}={}", key, urlencoding::encode(val)));
        first = false;
    };

    if let (Some(f), Some(t)) = (from, to) {
        add("from", &f.to_string());
        add("to", &t.to_string());
    } else if let Some(ref tr) = time_range {
        add("timeRange", tr);
    }
    if let Some(ref start) = start_date {
        add("startDate", start);
    }
    if let Some(ref end) = end_date {
        add("endDate", end);
    }

    match bridge.get::<StoreOverview>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            warn!(error = %e, "get_statistics failed");
            Ok(ApiResponse::from_bridge_error(e))
        }
    }
}

/// 获取销售报告（分页）
#[tauri::command]
pub async fn get_sales_report(
    bridge: State<'_, Arc<ClientBridge>>,
    time_range: String,
    page: Option<i32>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<ApiResponse<SalesReportResponse>, String> {
    let page = page.unwrap_or(1);
    let mut path = format!(
        "/api/statistics/sales-report?timeRange={}&page={}",
        urlencoding::encode(&time_range),
        page
    );
    if let Some(start) = start_date {
        path.push_str(&format!("&startDate={}", urlencoding::encode(&start)));
    }
    if let Some(end) = end_date {
        path.push_str(&format!("&endDate={}", urlencoding::encode(&end)));
    }

    match bridge.get::<SalesReportResponse>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            warn!(error = %e, "get_sales_report failed");
            Ok(ApiResponse::from_bridge_error(e))
        }
    }
}

// ============================================================================
// Red Flags
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemFlags {
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFlags {
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentFlags {
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorRedFlags {
    pub operator_id: i64,
    pub operator_name: String,
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
    pub total_flags: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagsResponse {
    pub item_flags: ItemFlags,
    pub order_flags: OrderFlags,
    pub payment_flags: PaymentFlags,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub operator_id: i64,
    pub operator_name: String,
    pub receipt_number: String,
    pub order_id: i64,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlagLogResponse {
    pub entries: Vec<RedFlagLogEntry>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

/// 获取 Red Flags 汇总数据
#[tauri::command]
pub async fn get_red_flags(
    bridge: State<'_, Arc<ClientBridge>>,
    from: i64,
    to: i64,
) -> Result<ApiResponse<RedFlagsResponse>, String> {
    let path = format!("/api/statistics/red-flags?from={}&to={}", from, to);
    match bridge.get::<RedFlagsResponse>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            warn!(from, to, error = %e, "get_red_flags failed");
            Ok(ApiResponse::from_bridge_error(e))
        }
    }
}

/// 获取 Red Flags 事件日志
#[tauri::command]
pub async fn get_red_flag_log(
    bridge: State<'_, Arc<ClientBridge>>,
    from: i64,
    to: i64,
    event_type: Option<String>,
    operator_id: Option<i64>,
    page: Option<i32>,
) -> Result<ApiResponse<RedFlagLogResponse>, String> {
    let mut path = format!("/api/statistics/red-flags/log?from={}&to={}", from, to);
    if let Some(ref et) = event_type {
        path.push_str(&format!("&event_type={}", et));
    }
    if let Some(op) = operator_id {
        path.push_str(&format!("&operator_id={}", op));
    }
    if let Some(p) = page {
        path.push_str(&format!("&page={}", p));
    }
    match bridge.get::<RedFlagLogResponse>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            warn!(from, to, error = %e, "get_red_flag_log failed");
            Ok(ApiResponse::from_bridge_error(e))
        }
    }
}

// ============================================================================
// Invoice List
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceListRow {
    pub id: i64,
    pub invoice_number: String,
    pub tipo_factura: String,
    pub source_type: String,
    pub source_pk: i64,
    pub total: f64,
    pub tax: f64,
    pub aeat_status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<InvoiceListRow>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

/// 获取发票列表（分页）
#[tauri::command]
pub async fn list_invoices(
    bridge: State<'_, Arc<ClientBridge>>,
    from: i64,
    to: i64,
    tipo: Option<String>,
    aeat_status: Option<String>,
    page: Option<i32>,
) -> Result<ApiResponse<InvoiceListResponse>, String> {
    let mut path = format!("/api/statistics/invoices?from={}&to={}", from, to);
    if let Some(t) = tipo {
        path.push_str(&format!("&tipo={}", urlencoding::encode(&t)));
    }
    if let Some(s) = aeat_status {
        path.push_str(&format!("&aeat_status={}", urlencoding::encode(&s)));
    }
    if let Some(p) = page {
        path.push_str(&format!("&page={}", p));
    }
    match bridge.get::<InvoiceListResponse>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => {
            warn!(from, to, error = %e, "list_invoices failed");
            Ok(ApiResponse::from_bridge_error(e))
        }
    }
}
