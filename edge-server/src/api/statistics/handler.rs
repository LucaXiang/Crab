//! Statistics API Handlers

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{Duration, Datelike};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::repository::StoreInfoRepository;
use crate::utils::AppResult;
use crate::utils::time;

// ============================================================================
// Response Types
// ============================================================================

/// Overview statistics
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesReportItem {
    pub order_id: String,
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
        "today" => {
            (cutoff_millis(today), cutoff_millis(today + Duration::days(1)))
        }
        "week" => {
            let weekday = today.weekday().num_days_from_monday();
            let week_start = today - Duration::days(weekday as i64);
            (cutoff_millis(week_start), cutoff_millis(today + Duration::days(1)))
        }
        "month" => {
            let month_start = today.with_day(1).unwrap_or(today);
            (cutoff_millis(month_start), cutoff_millis(today + Duration::days(1)))
        }
        "custom" => {
            if let (Some(s), Some(e)) = (custom_start, custom_end) {
                (parse_datetime(s), parse_datetime(e))
            } else {
                (cutoff_millis(today), cutoff_millis(today + Duration::days(1)))
            }
        }
        _ => {
            (cutoff_millis(today), cutoff_millis(today + Duration::days(1)))
        }
    }
}

/// Predefined colors for category chart
const CATEGORY_COLORS: &[&str] = &[
    "#3B82F6", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6",
    "#EC4899", "#06B6D4", "#84CC16", "#F97316", "#6366F1",
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
    let store_repo = StoreInfoRepository::new(state.db.clone());
    let cutoff = store_repo
        .get()
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

    // Query for overview stats
    let mut result = state.db
        .query(r#"
            -- Get all orders in time range (only needed fields)
            LET $all_orders = SELECT
                id, status, total_amount, paid_amount, guest_count,
                discount_amount, start_time, end_time
            FROM order
                WHERE end_time >= $start
                AND end_time < $end;

            -- Filter by status
            LET $completed = SELECT * FROM $all_orders WHERE status = 'COMPLETED';
            LET $void = SELECT * FROM $all_orders WHERE status = 'VOID';

            -- Calculate totals
            LET $total_revenue = math::sum($completed.total_amount) OR 0;
            LET $total_orders = count($completed);
            LET $total_customers = math::sum($completed.guest_count) OR 0;
            LET $avg_order = IF $total_orders > 0 THEN $total_revenue / $total_orders ELSE 0 END;
            LET $avg_guest_spend = IF $total_customers > 0 THEN $total_revenue / $total_customers ELSE 0 END;
            LET $voided_orders = count($void);
            LET $voided_amount = math::sum($void.total_amount) OR 0;
            LET $total_discount = math::sum($completed.discount_amount) OR 0;

            -- Payment breakdowns
            LET $completed_ids = (SELECT VALUE id FROM $completed);
            LET $payments = (
                SELECT out.method AS method, out.amount AS amount
                FROM has_payment
                WHERE in IN $completed_ids AND out.cancelled = false
            );
            LET $cash = math::sum((SELECT VALUE amount FROM $payments WHERE method = 'CASH')) OR 0;
            LET $card = math::sum((SELECT VALUE amount FROM $payments WHERE method = 'CARD')) OR 0;
            LET $other = $total_revenue - $cash - $card;

            -- Average dining time (minutes)
            LET $dining_times = (
                SELECT (end_time - start_time) / 60000 AS minutes
                FROM $completed
                WHERE end_time IS NOT NULL AND start_time IS NOT NULL
            );
            LET $avg_dining_time = math::mean($dining_times.minutes);

            RETURN {
                revenue: $total_revenue,
                orders: $total_orders,
                customers: $total_customers,
                average_order_value: $avg_order,
                cash_revenue: $cash,
                card_revenue: $card,
                other_revenue: $other,
                voided_orders: $voided_orders,
                voided_amount: $voided_amount,
                total_discount: $total_discount,
                avg_guest_spend: $avg_guest_spend,
                avg_dining_time: $avg_dining_time
            }
        "#)
        .bind(("start", start_dt))
        .bind(("end", end_dt))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let overview: OverviewStats = result.take::<Option<OverviewStats>>(18)
        .map_err(crate::db::repository::surreal_err_to_app)?
        .unwrap_or(OverviewStats {
            revenue: 0.0,
            orders: 0,
            customers: 0,
            average_order_value: 0.0,
            cash_revenue: 0.0,
            card_revenue: 0.0,
            other_revenue: 0.0,
            voided_orders: 0,
            voided_amount: 0.0,
            total_discount: 0.0,
            avg_guest_spend: 0.0,
            avg_dining_time: None,
        });

    // Query for revenue trend (hourly for today, daily for week/month)
    let trend_query = if query.time_range == "today" {
        // Hourly trend: group by hour bucket
        r#"
            SELECT
                time::format(time::from::unix(end_time / 1000), '%H:00') AS time,
                math::sum(total_amount) AS value
            FROM order
            WHERE status = 'COMPLETED'
            AND end_time >= $start
            AND end_time < $end
            GROUP BY time
            ORDER BY time
        "#
    } else {
        // Daily trend: group by date bucket
        r#"
            SELECT
                time::format(time::from::unix(end_time / 1000), '%m-%d') AS time,
                math::sum(total_amount) AS value
            FROM order
            WHERE status = 'COMPLETED'
            AND end_time >= $start
            AND end_time < $end
            GROUP BY time
            ORDER BY time
        "#
    };

    let mut trend_result = state.db
        .query(trend_query)
        .bind(("start", start_dt))
        .bind(("end", end_dt))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let revenue_trend: Vec<RevenueTrendPoint> = trend_result.take(0)
        .map_err(crate::db::repository::surreal_err_to_app)?;

    // Query for category sales
    let mut category_result = state.db
        .query(r#"
            LET $completed_ids = (
                SELECT VALUE id FROM order
                WHERE status = 'COMPLETED'
                AND end_time >= $start
                AND end_time < $end
            );

            SELECT
                (out.category_name OR "未分类") AS name,
                math::sum(out.line_total) AS value
            FROM has_item
            WHERE in IN $completed_ids
            GROUP BY name
            ORDER BY value DESC
            LIMIT 10
        "#)
        .bind(("start", start_dt))
        .bind(("end", end_dt))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    #[derive(Deserialize)]
    struct CategoryRaw {
        name: Option<String>,
        value: f64,
    }

    let category_raw: Vec<CategoryRaw> = category_result.take(1)
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let category_sales: Vec<CategorySale> = category_raw
        .into_iter()
        .enumerate()
        .map(|(i, c)| CategorySale {
            name: c.name.unwrap_or_else(|| "Unknown".to_string()),
            value: c.value,
            color: CATEGORY_COLORS.get(i % CATEGORY_COLORS.len())
                .unwrap_or(&"#6B7280")
                .to_string(),
        })
        .collect();

    // Query for top products
    let mut product_result = state.db
        .query(r#"
            LET $completed_ids = (
                SELECT VALUE id FROM order
                WHERE status = 'COMPLETED'
                AND end_time >= $start
                AND end_time < $end
            );

            SELECT
                out.name AS name,
                math::sum(out.quantity) AS sales
            FROM has_item
            WHERE in IN $completed_ids
            GROUP BY out.name
            ORDER BY sales DESC
            LIMIT 10
        "#)
        .bind(("start", start_dt))
        .bind(("end", end_dt))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let top_products: Vec<TopProduct> = product_result.take(1)
        .map_err(crate::db::repository::surreal_err_to_app)?;

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
    let store_repo = StoreInfoRepository::new(state.db.clone());
    let cutoff = store_repo
        .get()
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

    // Workaround: SurrealDB embedded mode (kv-rocksdb) drops the first record
    // when LIMIT is combined with computed fields like <string>id on indexed fields.
    // Dataset is bounded by time range, so in-memory pagination is fine.
    let mut data_result = state.db
        .query(r#"
            SELECT
                <string>id AS order_id,
                receipt_number,
                time::format(time::from::unix(end_time / 1000), '%Y-%m-%d %H:%M') AS date,
                total_amount AS total,
                string::uppercase(status) AS status,
                end_time
            FROM order
            WHERE end_time >= $start
            AND end_time < $end
            ORDER BY end_time DESC
        "#)
        .bind(("start", start_dt))
        .bind(("end", end_dt))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let all_items: Vec<SalesReportItem> = data_result.take(0)
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let total = all_items.len() as i32;
    let total_pages = if total > 0 { (total + page_size - 1) / page_size } else { 1 };
    let items: Vec<SalesReportItem> = all_items
        .into_iter()
        .skip(offset as usize)
        .take(page_size as usize)
        .collect();

    Ok(Json(SalesReportResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}
