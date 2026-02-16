//! Statistics API Handlers

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{Datelike, Duration};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::repository::store_info;
use crate::utils::time;
use crate::utils::{AppError, AppResult};

// ============================================================================
// Response Types
// ============================================================================

/// Overview statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

/// Revenue trend data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueTrendPoint {
    pub time: String,
    pub value: f64,
}

/// Category sales data
#[derive(Debug, Clone, Serialize)]
pub struct CategorySale {
    pub name: String,
    pub value: f64,
    pub color: String,
}

/// Top product data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProduct {
    pub name: String,
    pub sales: i32,
}

/// Full statistics response
#[derive(Debug, Clone, Serialize)]
pub struct StatisticsResponse {
    pub overview: OverviewStats,
    pub revenue_trend: Vec<RevenueTrendPoint>,
    pub category_sales: Vec<CategorySale>,
    pub top_products: Vec<TopProduct>,
}

/// Sales report item
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SalesReportItem {
    pub order_id: i64,
    pub receipt_number: Option<String>,
    pub date: String,
    pub total: f64,
    pub status: String,
}

/// Sales report response
#[derive(Debug, Clone, Serialize)]
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
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StatisticsQuery {
    #[serde(rename = "timeRange")]
    pub time_range: String,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SalesReportQuery {
    #[serde(rename = "timeRange")]
    pub time_range: String,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
}

fn default_page() -> i32 {
    1
}

// ============================================================================
// Time Range Calculation
// ============================================================================

/// Calculate business day time range based on cutoff time
/// Returns (start_millis, end_millis) as Unix timestamp milliseconds
fn calculate_time_range(
    time_range: &str,
    cutoff: &str,
    custom_start: Option<&str>,
    custom_end: Option<&str>,
    tz: chrono_tz::Tz,
) -> (i64, i64) {
    let cutoff_time = time::parse_cutoff(cutoff);
    let today = time::current_business_date(cutoff_time, tz);

    let cutoff_millis = |date| time::date_cutoff_millis(date, cutoff_time, tz);

    let parse_datetime = |s: &str| -> i64 {
        if s.contains('T') {
            let normalized = if s.ends_with('Z') || s.contains('+') {
                s.to_string()
            } else if s.len() == 16 {
                format!("{}:00Z", s)
            } else {
                format!("{}Z", s)
            };
            chrono::DateTime::parse_from_rfc3339(&normalized)
                .or_else(|_| chrono::DateTime::parse_from_rfc3339(&format!("{}Z", s)))
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0)
        } else {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(&cutoff_millis)
                .unwrap_or(0)
        }
    };

    match time_range {
        "today" => (
            cutoff_millis(today),
            cutoff_millis(today + Duration::days(1)),
        ),
        "week" => {
            let weekday = today.weekday().num_days_from_monday();
            let week_start = today - Duration::days(weekday as i64);
            (
                cutoff_millis(week_start),
                cutoff_millis(today + Duration::days(1)),
            )
        }
        "month" => {
            let month_start = today.with_day(1).unwrap_or(today);
            (
                cutoff_millis(month_start),
                cutoff_millis(today + Duration::days(1)),
            )
        }
        "custom" => {
            if let (Some(s), Some(e)) = (custom_start, custom_end) {
                (parse_datetime(s), parse_datetime(e))
            } else {
                (
                    cutoff_millis(today),
                    cutoff_millis(today + Duration::days(1)),
                )
            }
        }
        _ => (
            cutoff_millis(today),
            cutoff_millis(today + Duration::days(1)),
        ),
    }
}

/// Predefined colors for category chart
const CATEGORY_COLORS: &[&str] = &[
    "#3B82F6", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6", "#EC4899", "#06B6D4", "#84CC16",
    "#F97316", "#6366F1",
];

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/statistics - Get statistics overview
pub async fn get_statistics(
    State(state): State<ServerState>,
    Query(query): Query<StatisticsQuery>,
) -> AppResult<Json<StatisticsResponse>> {
    // Get business day cutoff from store info
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or_else(|| "02:00".to_string());

    let (start_dt, end_dt) = calculate_time_range(
        &query.time_range,
        &cutoff,
        query.start_date.as_deref(),
        query.end_date.as_deref(),
        state.config.timezone,
    );

    tracing::debug!(
        time_range = %query.time_range,
        start = %start_dt,
        end = %end_dt,
        cutoff = %cutoff,
        "Fetching statistics"
    );

    let pool = &state.pool;

    // Overview: single aggregate query (was 7 separate queries)
    let (revenue, total_orders, total_customers, voided_orders, voided_amount, total_discount, avg_dining_time): (f64, i32, i32, i32, f64, f64, Option<f64>) = sqlx::query_as(
        "SELECT \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN total_amount ELSE 0.0 END), 0.0), \
            CAST(COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) AS INTEGER), \
            CAST(COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN guest_count ELSE 0 END), 0) AS INTEGER), \
            CAST(COUNT(CASE WHEN status = 'VOID' THEN 1 END) AS INTEGER), \
            COALESCE(SUM(CASE WHEN status = 'VOID' THEN total_amount ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN discount_amount ELSE 0.0 END), 0.0), \
            AVG(CASE WHEN status = 'COMPLETED' AND end_time IS NOT NULL AND start_time IS NOT NULL \
                THEN CAST((end_time - start_time) AS REAL) / 60000.0 END) \
         FROM archived_order WHERE end_time >= ?1 AND end_time < ?2",
    )
    .bind(start_dt).bind(end_dt)
    .fetch_one(pool).await.map_err(|e| AppError::database(e.to_string()))?;

    let average_order_value = if total_orders > 0 {
        revenue / total_orders as f64
    } else {
        0.0
    };
    let avg_guest_spend = if total_customers > 0 {
        revenue / total_customers as f64
    } else {
        0.0
    };

    // Payment breakdown: single query (was 2 separate queries)
    let (cash_revenue, card_revenue): (f64, f64) = sqlx::query_as(
        "SELECT \
            COALESCE(SUM(CASE WHEN p.method = 'CASH' THEN p.amount ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN p.method = 'CARD' THEN p.amount ELSE 0.0 END), 0.0) \
         FROM archived_order_payment p \
         JOIN archived_order o ON p.order_pk = o.id \
         WHERE o.end_time >= ?1 AND o.end_time < ?2 AND o.status = 'COMPLETED' AND p.cancelled = 0",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let other_revenue = revenue - cash_revenue - card_revenue;

    let overview = OverviewStats {
        revenue,
        orders: total_orders,
        customers: total_customers,
        average_order_value,
        cash_revenue,
        card_revenue,
        other_revenue,
        voided_orders,
        voided_amount,
        total_discount,
        avg_guest_spend,
        avg_dining_time,
    };

    // Revenue trend
    let revenue_trend = if query.time_range == "today" {
        // Hourly trend
        let rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT PRINTF('%02d:00', (end_time / 1000 / 3600) % 24) AS time, COALESCE(SUM(total_amount), 0.0) AS value FROM archived_order WHERE status = 'COMPLETED' AND end_time >= ?1 AND end_time < ?2 GROUP BY time ORDER BY time",
        )
        .bind(start_dt).bind(end_dt)
        .fetch_all(pool).await.map_err(|e| AppError::database(e.to_string()))?;

        rows.into_iter()
            .map(|(t, v)| RevenueTrendPoint { time: t, value: v })
            .collect()
    } else {
        // Daily trend
        let rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT STRFTIME('%m-%d', end_time / 1000, 'unixepoch') AS time, COALESCE(SUM(total_amount), 0.0) AS value FROM archived_order WHERE status = 'COMPLETED' AND end_time >= ?1 AND end_time < ?2 GROUP BY time ORDER BY time",
        )
        .bind(start_dt).bind(end_dt)
        .fetch_all(pool).await.map_err(|e| AppError::database(e.to_string()))?;

        rows.into_iter()
            .map(|(t, v)| RevenueTrendPoint { time: t, value: v })
            .collect()
    };

    // Category sales from archived_order_item
    let category_raw: Vec<(Option<String>, f64)> = sqlx::query_as(
        "SELECT COALESCE(i.category_name, 'Unknown') AS name, COALESCE(SUM(i.line_total), 0.0) AS value FROM archived_order_item i JOIN archived_order o ON i.order_pk = o.id WHERE o.status = 'COMPLETED' AND o.end_time >= ?1 AND o.end_time < ?2 GROUP BY name ORDER BY value DESC LIMIT 10",
    )
    .bind(start_dt).bind(end_dt)
    .fetch_all(pool).await.map_err(|e| AppError::database(e.to_string()))?;

    let category_sales: Vec<CategorySale> = category_raw
        .into_iter()
        .enumerate()
        .map(|(i, (name, value))| CategorySale {
            name: name.unwrap_or_else(|| "Unknown".to_string()),
            value,
            color: CATEGORY_COLORS
                .get(i % CATEGORY_COLORS.len())
                .unwrap_or(&"#6B7280")
                .to_string(),
        })
        .collect();

    // Top products from archived_order_item
    let top_products: Vec<TopProduct> = sqlx::query_as::<_, (String, i32)>(
        "SELECT i.name, COALESCE(SUM(i.quantity), 0) AS sales FROM archived_order_item i JOIN archived_order o ON i.order_pk = o.id WHERE o.status = 'COMPLETED' AND o.end_time >= ?1 AND o.end_time < ?2 GROUP BY i.name ORDER BY sales DESC LIMIT 10",
    )
    .bind(start_dt).bind(end_dt)
    .fetch_all(pool).await.map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(name, sales)| TopProduct { name, sales })
    .collect();

    Ok(Json(StatisticsResponse {
        overview,
        revenue_trend,
        category_sales,
        top_products,
    }))
}

/// GET /api/statistics/sales-report - Get paginated sales report
pub async fn get_sales_report(
    State(state): State<ServerState>,
    Query(query): Query<SalesReportQuery>,
) -> AppResult<Json<SalesReportResponse>> {
    // Get business day cutoff from store info
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or_else(|| "02:00".to_string());

    let (start_dt, end_dt) = calculate_time_range(
        &query.time_range,
        &cutoff,
        query.start_date.as_deref(),
        query.end_date.as_deref(),
        state.config.timezone,
    );

    let page = query.page.max(1);
    let page_size = 10;
    let offset = (page - 1) * page_size;

    let pool = &state.pool;

    let total_i64: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM archived_order WHERE end_time >= ?1 AND end_time < ?2",
        start_dt,
        end_dt,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    let total = i32::try_from(total_i64).unwrap_or(i32::MAX);

    let total_pages = if total > 0 {
        (total + page_size - 1) / page_size
    } else {
        1
    };

    let items: Vec<SalesReportItem> = sqlx::query_as(
        "SELECT id AS order_id, receipt_number, STRFTIME('%Y-%m-%d %H:%M', end_time / 1000, 'unixepoch') AS date, total_amount AS total, UPPER(status) AS status FROM archived_order WHERE end_time >= ?1 AND end_time < ?2 ORDER BY end_time DESC LIMIT ?3 OFFSET ?4",
    )
    .bind(start_dt).bind(end_dt)
    .bind(page_size).bind(offset)
    .fetch_all(pool).await.unwrap_or_default();

    Ok(Json(SalesReportResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}
