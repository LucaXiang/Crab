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
// Response Types — aligned with Console's StoreOverview
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct StoreOverview {
    pub revenue: f64,
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

#[derive(Debug, Clone, Serialize)]
pub struct RevenueTrendPoint {
    pub hour: i32,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyTrendPoint {
    pub date: String,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentBreakdownEntry {
    pub method: String,
    pub amount: f64,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaxBreakdownEntry {
    pub tax_rate: f64,
    pub base_amount: f64,
    pub tax_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategorySaleEntry {
    pub name: String,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopProductEntry {
    pub name: String,
    pub quantity: i32,
    pub revenue: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagSaleEntry {
    pub name: String,
    pub color: Option<String>,
    pub revenue: f64,
    pub quantity: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefundMethodEntry {
    pub method: String,
    pub amount: f64,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceTypeEntry {
    pub service_type: String,
    pub revenue: f64,
    pub orders: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ZoneSaleEntry {
    pub zone_name: String,
    pub revenue: f64,
    pub orders: i32,
    pub guests: i32,
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
    pub time_range: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    /// Unix millis start (alternative to timeRange presets)
    pub from: Option<i64>,
    /// Unix millis end
    pub to: Option<i64>,
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
    cutoff: i32,
    custom_start: Option<&str>,
    custom_end: Option<&str>,
    tz: chrono_tz::Tz,
) -> (i64, i64) {
    let cutoff_time = time::cutoff_to_time(cutoff);
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

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/statistics - Get store overview statistics
pub async fn get_statistics(
    State(state): State<ServerState>,
    Query(query): Query<StatisticsQuery>,
) -> AppResult<Json<StoreOverview>> {
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or(0);

    // Support both from/to millis and timeRange presets
    let (start_dt, end_dt) = if let (Some(from), Some(to)) = (query.from, query.to) {
        (from, to)
    } else {
        let time_range = query.time_range.as_deref().unwrap_or("today");
        calculate_time_range(
            time_range,
            cutoff,
            query.start_date.as_deref(),
            query.end_date.as_deref(),
            state.config.timezone,
        )
    };

    tracing::debug!(start = %start_dt, end = %end_dt, cutoff, "Fetching statistics");

    let pool = &state.pool;

    // ── Overview aggregate ──
    #[allow(clippy::type_complexity)]
    let (revenue, total_orders, guests, voided_orders, voided_amount, loss_orders, loss_amount, total_discount, total_surcharge, total_tax, avg_dining_time): (f64, i32, i32, i32, f64, i32, f64, f64, f64, f64, Option<f64>) = sqlx::query_as(
        "SELECT \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN total_amount ELSE 0.0 END), 0.0), \
            CAST(COUNT(CASE WHEN status = 'COMPLETED' THEN 1 END) AS INTEGER), \
            CAST(COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN guest_count ELSE 0 END), 0) AS INTEGER), \
            CAST(COUNT(CASE WHEN status = 'VOID' AND (void_type IS NULL OR void_type != 'LOSS_SETTLED') THEN 1 END) AS INTEGER), \
            COALESCE(SUM(CASE WHEN status = 'VOID' AND (void_type IS NULL OR void_type != 'LOSS_SETTLED') THEN total_amount ELSE 0.0 END), 0.0), \
            CAST(COUNT(CASE WHEN status = 'VOID' AND void_type = 'LOSS_SETTLED' THEN 1 END) AS INTEGER), \
            COALESCE(SUM(CASE WHEN status = 'VOID' AND void_type = 'LOSS_SETTLED' THEN COALESCE(loss_amount, 0.0) ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN discount_amount ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN surcharge_amount ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN tax ELSE 0.0 END), 0.0), \
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
    let per_guest_spend = if guests > 0 {
        revenue / guests as f64
    } else {
        0.0
    };

    // ── Avg items per order ──
    let avg_items_per_order: f64 = sqlx::query_scalar(
        "SELECT COALESCE(AVG(cnt), 0.0) FROM (\
            SELECT COUNT(*) AS cnt FROM archived_order_item i \
            JOIN archived_order o ON i.order_pk = o.id \
            WHERE o.status = 'COMPLETED' AND o.end_time >= ?1 AND o.end_time < ?2 \
            GROUP BY o.id\
        )",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_one(pool)
    .await
    .unwrap_or(0.0);

    // ── Payment breakdown ──
    let payment_breakdown: Vec<PaymentBreakdownEntry> = sqlx::query_as::<_, (String, f64, i32)>(
        "SELECT p.method, COALESCE(SUM(p.amount), 0.0), CAST(COUNT(*) AS INTEGER) \
         FROM archived_order_payment p \
         JOIN archived_order o ON p.order_pk = o.id \
         WHERE o.end_time >= ?1 AND o.end_time < ?2 AND o.status = 'COMPLETED' AND p.cancelled = 0 \
         GROUP BY p.method ORDER BY SUM(p.amount) DESC",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(method, amount, count)| PaymentBreakdownEntry {
        method,
        amount,
        count,
    })
    .collect();

    // ── Refund totals + method breakdown ──
    let (refund_amount, refund_count): (f64, i32) = sqlx::query_as(
        "SELECT COALESCE(SUM(total_credit), 0.0), CAST(COUNT(*) AS INTEGER) \
         FROM credit_note WHERE created_at >= ?1 AND created_at < ?2",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let refund_method_breakdown: Vec<RefundMethodEntry> = sqlx::query_as::<_, (String, f64, i32)>(
        "SELECT refund_method, COALESCE(SUM(total_credit), 0.0), CAST(COUNT(*) AS INTEGER) \
         FROM credit_note WHERE created_at >= ?1 AND created_at < ?2 \
         GROUP BY refund_method ORDER BY SUM(total_credit) DESC",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(method, amount, count)| RefundMethodEntry {
        method,
        amount,
        count,
    })
    .collect();

    // ── Revenue trend (hourly) ──
    let revenue_trend: Vec<RevenueTrendPoint> = sqlx::query_as::<_, (i32, f64, i64)>(
        "SELECT CAST((end_time / 1000 / 3600) % 24 AS INTEGER) AS hour, \
            COALESCE(SUM(total_amount), 0.0), COUNT(*) \
         FROM archived_order \
         WHERE status = 'COMPLETED' AND end_time >= ?1 AND end_time < ?2 \
         GROUP BY hour ORDER BY hour",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(hour, revenue, orders)| RevenueTrendPoint {
        hour,
        revenue,
        orders,
    })
    .collect();

    // ── Daily trend ──
    let daily_trend: Vec<DailyTrendPoint> = sqlx::query_as::<_, (String, f64, i64)>(
        "SELECT STRFTIME('%Y-%m-%d', end_time / 1000, 'unixepoch') AS date, \
            COALESCE(SUM(total_amount), 0.0), COUNT(*) \
         FROM archived_order \
         WHERE status = 'COMPLETED' AND end_time >= ?1 AND end_time < ?2 \
         GROUP BY date ORDER BY date",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(date, revenue, orders)| DailyTrendPoint {
        date,
        revenue,
        orders,
    })
    .collect();

    // ── Tax breakdown (from item-level tax_rate) ──
    let tax_breakdown: Vec<TaxBreakdownEntry> = sqlx::query_as::<_, (f64, f64, f64)>(
        "SELECT CAST(i.tax_rate AS REAL) AS rate, \
            COALESCE(SUM(i.line_total - i.tax), 0.0) AS base, \
            COALESCE(SUM(i.tax), 0.0) AS tax_amt \
         FROM archived_order_item i \
         JOIN archived_order o ON i.order_pk = o.id \
         WHERE o.status = 'COMPLETED' AND o.end_time >= ?1 AND o.end_time < ?2 \
         GROUP BY i.tax_rate ORDER BY rate",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(tax_rate, base_amount, tax_amount)| TaxBreakdownEntry {
        tax_rate,
        base_amount,
        tax_amount,
    })
    .collect();

    // ── Category sales ──
    let category_sales: Vec<CategorySaleEntry> = sqlx::query_as::<_, (String, f64)>(
        "SELECT COALESCE(i.category_name, 'Unknown'), COALESCE(SUM(i.line_total), 0.0) \
         FROM archived_order_item i \
         JOIN archived_order o ON i.order_pk = o.id \
         WHERE o.status = 'COMPLETED' AND o.end_time >= ?1 AND o.end_time < ?2 \
         GROUP BY i.category_name ORDER BY SUM(i.line_total) DESC LIMIT 10",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(name, revenue)| CategorySaleEntry { name, revenue })
    .collect();

    // ── Top products (with revenue) ──
    let top_products: Vec<TopProductEntry> = sqlx::query_as::<_, (String, i32, f64)>(
        "SELECT i.name, CAST(COALESCE(SUM(i.quantity), 0) AS INTEGER), COALESCE(SUM(i.line_total), 0.0) \
         FROM archived_order_item i \
         JOIN archived_order o ON i.order_pk = o.id \
         WHERE o.status = 'COMPLETED' AND o.end_time >= ?1 AND o.end_time < ?2 \
         GROUP BY i.name ORDER BY SUM(i.line_total) DESC LIMIT 10",
    )
    .bind(start_dt).bind(end_dt)
    .fetch_all(pool).await.map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(name, quantity, revenue)| TopProductEntry { name, quantity, revenue })
    .collect();

    // ── Tag sales (join through product_tag) ──
    let tag_sales: Vec<TagSaleEntry> = sqlx::query_as::<_, (String, Option<String>, f64, i32)>(
        "SELECT t.name, t.color, COALESCE(SUM(i.line_total), 0.0), CAST(COALESCE(SUM(i.quantity), 0) AS INTEGER) \
         FROM archived_order_item i \
         JOIN archived_order o ON i.order_pk = o.id \
         JOIN product_tag pt ON CAST(i.spec AS INTEGER) = pt.product_id \
         JOIN tag t ON pt.tag_id = t.id \
         WHERE o.status = 'COMPLETED' AND o.end_time >= ?1 AND o.end_time < ?2 \
         GROUP BY t.id ORDER BY SUM(i.line_total) DESC LIMIT 20",
    )
    .bind(start_dt).bind(end_dt)
    .fetch_all(pool).await.unwrap_or_default()
    .into_iter()
    .map(|(name, color, revenue, quantity)| TagSaleEntry { name, color, revenue, quantity })
    .collect();

    // ── Service type breakdown ──
    let service_type_breakdown: Vec<ServiceTypeEntry> = sqlx::query_as::<_, (String, f64, i32)>(
        "SELECT COALESCE(service_type, CASE WHEN is_retail = 1 THEN 'Retail' ELSE 'DineIn' END), \
            COALESCE(SUM(total_amount), 0.0), CAST(COUNT(*) AS INTEGER) \
         FROM archived_order \
         WHERE status = 'COMPLETED' AND end_time >= ?1 AND end_time < ?2 \
         GROUP BY COALESCE(service_type, CASE WHEN is_retail = 1 THEN 'Retail' ELSE 'DineIn' END) \
         ORDER BY SUM(total_amount) DESC",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(service_type, revenue, orders)| ServiceTypeEntry {
        service_type,
        revenue,
        orders,
    })
    .collect();

    // ── Zone sales ──
    let zone_sales: Vec<ZoneSaleEntry> = sqlx::query_as::<_, (String, f64, i32, i32)>(
        "SELECT COALESCE(zone_name, 'Unknown'), COALESCE(SUM(total_amount), 0.0), \
            CAST(COUNT(*) AS INTEGER), CAST(COALESCE(SUM(guest_count), 0) AS INTEGER) \
         FROM archived_order \
         WHERE status = 'COMPLETED' AND end_time >= ?1 AND end_time < ?2 AND zone_name IS NOT NULL \
         GROUP BY zone_name ORDER BY SUM(total_amount) DESC",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(zone_name, revenue, orders, guests)| ZoneSaleEntry {
        zone_name,
        revenue,
        orders,
        guests,
    })
    .collect();

    Ok(Json(StoreOverview {
        revenue,
        orders: total_orders,
        guests,
        average_order_value,
        per_guest_spend,
        average_dining_minutes: avg_dining_time.unwrap_or(0.0),
        total_tax,
        total_discount,
        total_surcharge,
        avg_items_per_order,
        voided_orders,
        voided_amount,
        loss_orders,
        loss_amount,
        refund_count,
        refund_amount,
        revenue_trend,
        daily_trend,
        payment_breakdown,
        tax_breakdown,
        category_sales,
        top_products,
        tag_sales,
        refund_method_breakdown,
        service_type_breakdown,
        zone_sales,
    }))
}

/// GET /api/statistics/sales-report - Get paginated sales report
pub async fn get_sales_report(
    State(state): State<ServerState>,
    Query(query): Query<SalesReportQuery>,
) -> AppResult<Json<SalesReportResponse>> {
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or(0);

    let (start_dt, end_dt) = calculate_time_range(
        &query.time_range,
        cutoff,
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
