//! Statistics Commands
//!
//! 数据统计相关的 Tauri 命令

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::core::response::ApiResponse;
use crate::core::ClientBridge;

// ============================================================================
// Response Types (与前端 TypeScript 类型对齐)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewStats {
    pub revenue: f64,
    pub orders: i32,
    pub customers: i32,
    pub average_order_value: f64,
    pub cash_revenue: f64,
    pub card_revenue: f64,
    pub other_revenue: f64,
    pub voided_orders: i32,
    pub voided_amount: f64,
    pub total_discount: f64,
    pub avg_guest_spend: f64,
    pub avg_dining_time: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueTrendPoint {
    pub time: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySale {
    pub name: String,
    pub value: f64,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProduct {
    pub name: String,
    pub sales: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticsResponse {
    pub overview: OverviewStats,
    pub revenue_trend: Vec<RevenueTrendPoint>,
    pub category_sales: Vec<CategorySale>,
    pub top_products: Vec<TopProduct>,
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

/// 获取统计数据
#[tauri::command]
pub async fn get_statistics(
    bridge: State<'_, Arc<ClientBridge>>,
    time_range: String,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<ApiResponse<StatisticsResponse>, String> {
    // Build query string
    let mut path = format!(
        "/api/statistics?timeRange={}",
        urlencoding::encode(&time_range)
    );
    if let Some(start) = start_date {
        path.push_str(&format!("&startDate={}", urlencoding::encode(&start)));
    }
    if let Some(end) = end_date {
        path.push_str(&format!("&endDate={}", urlencoding::encode(&end)));
    }

    match bridge.get::<StatisticsResponse>(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
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
    // Build query string
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
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}
