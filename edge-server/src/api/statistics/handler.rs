//! Statistics API Handlers

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{Datelike, Duration};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::repository::{invoice, store_info};
use crate::utils::time;
use crate::utils::{AppError, AppResult};

// ============================================================================
// Response Types — aligned with Console's StoreOverview
// ============================================================================

#[derive(Debug, Clone, Serialize)]
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
    pub discount_breakdown: Vec<AdjustmentEntry>,
    pub surcharge_breakdown: Vec<AdjustmentEntry>,
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
    pub is_retail: bool,
    pub revenue: f64,
    pub orders: i32,
    pub guests: i32,
}

/// Discount or surcharge line item in the breakdown
#[derive(Debug, Clone, Serialize)]
pub struct AdjustmentEntry {
    /// Display name (rule name, or i18n key like "item_manual")
    pub name: String,
    /// Source type: "item_manual", "item_rule", "mg", "order_manual", "order_rule"
    pub source: String,
    pub amount: f64,
    pub order_count: i32,
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
        "yesterday" => {
            let yesterday = today - Duration::days(1);
            (cutoff_millis(yesterday), cutoff_millis(today))
        }
        "this_week" | "week" => {
            let weekday = today.weekday().num_days_from_monday();
            let week_start = today - Duration::days(weekday as i64);
            (
                cutoff_millis(week_start),
                cutoff_millis(today + Duration::days(1)),
            )
        }
        "this_month" | "month" => {
            let month_start = today.with_day(1).unwrap_or(today);
            (
                cutoff_millis(month_start),
                cutoff_millis(today + Duration::days(1)),
            )
        }
        "last_month" => {
            let this_month_start = today.with_day(1).unwrap_or(today);
            let last_month_start = (this_month_start - Duration::days(1))
                .with_day(1)
                .unwrap_or(this_month_start - Duration::days(28));
            (
                cutoff_millis(last_month_start),
                cutoff_millis(this_month_start),
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
    // COMPLETED + is_voided=0 → active revenue; COMPLETED + is_voided=1 → anulacion
    #[allow(clippy::type_complexity)]
    let (revenue, total_orders, guests, voided_orders, voided_amount, loss_orders, loss_amount, total_discount, total_surcharge, total_tax, avg_dining_time, anulacion_count, anulacion_amount): (f64, i32, i32, i32, f64, i32, f64, f64, f64, f64, Option<f64>, i32, f64) = sqlx::query_as(
        "SELECT \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided = 0 THEN total_amount ELSE 0.0 END), 0.0), \
            CAST(COUNT(CASE WHEN status = 'COMPLETED' AND is_voided = 0 THEN 1 END) AS INTEGER), \
            CAST(COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided = 0 THEN guest_count ELSE 0 END), 0) AS INTEGER), \
            CAST(COUNT(CASE WHEN status = 'VOID' AND (void_type IS NULL OR void_type != 'LOSS_SETTLED') THEN 1 END) AS INTEGER), \
            COALESCE(SUM(CASE WHEN status = 'VOID' AND (void_type IS NULL OR void_type != 'LOSS_SETTLED') THEN total_amount ELSE 0.0 END), 0.0), \
            CAST(COUNT(CASE WHEN status = 'VOID' AND void_type = 'LOSS_SETTLED' THEN 1 END) AS INTEGER), \
            COALESCE(SUM(CASE WHEN status = 'VOID' AND void_type = 'LOSS_SETTLED' THEN COALESCE(loss_amount, 0.0) ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided = 0 THEN discount_amount ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided = 0 THEN surcharge_amount ELSE 0.0 END), 0.0), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided = 0 THEN tax ELSE 0.0 END), 0.0), \
            AVG(CASE WHEN status = 'COMPLETED' AND is_voided = 0 AND end_time IS NOT NULL AND start_time IS NOT NULL \
                THEN CAST((end_time - start_time) AS REAL) / 60000.0 END), \
            CAST(COUNT(CASE WHEN status = 'COMPLETED' AND is_voided = 1 THEN 1 END) AS INTEGER), \
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided = 1 THEN total_amount ELSE 0.0 END), 0.0) \
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
            WHERE o.status = 'COMPLETED' AND o.is_voided = 0 AND o.end_time >= ?1 AND o.end_time < ?2 \
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
         WHERE o.end_time >= ?1 AND o.end_time < ?2 AND o.status = 'COMPLETED' AND o.is_voided = 0 AND p.cancelled = 0 \
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
    .map_err(|e| AppError::database(e.to_string()))?
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
         WHERE status = 'COMPLETED' AND is_voided = 0 AND end_time >= ?1 AND end_time < ?2 \
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
         WHERE status = 'COMPLETED' AND is_voided = 0 AND end_time >= ?1 AND end_time < ?2 \
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
         WHERE o.status = 'COMPLETED' AND o.is_voided = 0 AND o.end_time >= ?1 AND o.end_time < ?2 \
         GROUP BY i.tax_rate ORDER BY rate",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
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
         WHERE o.status = 'COMPLETED' AND o.is_voided = 0 AND o.end_time >= ?1 AND o.end_time < ?2 \
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
         WHERE o.status = 'COMPLETED' AND o.is_voided = 0 AND o.end_time >= ?1 AND o.end_time < ?2 \
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
         WHERE o.status = 'COMPLETED' AND o.is_voided = 0 AND o.end_time >= ?1 AND o.end_time < ?2 \
         GROUP BY t.id ORDER BY SUM(i.line_total) DESC LIMIT 20",
    )
    .bind(start_dt).bind(end_dt)
    .fetch_all(pool).await.map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(name, color, revenue, quantity)| TagSaleEntry { name, color, revenue, quantity })
    .collect();

    // ── Service type breakdown ──
    let service_type_breakdown: Vec<ServiceTypeEntry> = sqlx::query_as::<_, (String, f64, i32)>(
        "SELECT COALESCE(service_type, CASE WHEN is_retail = 1 THEN 'TAKEOUT' ELSE 'DINE_IN' END), \
            COALESCE(SUM(total_amount), 0.0), CAST(COUNT(*) AS INTEGER) \
         FROM archived_order \
         WHERE status = 'COMPLETED' AND is_voided = 0 AND end_time >= ?1 AND end_time < ?2 \
         GROUP BY COALESCE(service_type, CASE WHEN is_retail = 1 THEN 'TAKEOUT' ELSE 'DINE_IN' END) \
         ORDER BY SUM(total_amount) DESC",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(service_type, revenue, orders)| ServiceTypeEntry {
        service_type,
        revenue,
        orders,
    })
    .collect();

    // ── Zone sales ──
    let zone_sales: Vec<ZoneSaleEntry> = sqlx::query_as::<_, (String, bool, f64, i32, i32)>(
        "SELECT COALESCE(NULLIF(zone_name, ''), CASE WHEN is_retail = 1 THEN 'Retail' ELSE 'Default' END), \
            is_retail, COALESCE(SUM(total_amount), 0.0), \
            CAST(COUNT(*) AS INTEGER), CAST(COALESCE(SUM(guest_count), 0) AS INTEGER) \
         FROM archived_order \
         WHERE status = 'COMPLETED' AND is_voided = 0 AND end_time >= ?1 AND end_time < ?2 \
         GROUP BY 1, is_retail ORDER BY SUM(total_amount) DESC",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .into_iter()
    .map(|(zone_name, is_retail, revenue, orders, guests)| ZoneSaleEntry {
        zone_name,
        is_retail,
        revenue,
        orders,
        guests,
    })
    .collect();

    // ── Discount & Surcharge Breakdown (from normalized archived_order_adjustment) ──
    // source_key uses same mapping as cloud: item_pk IS NULL → order level, else item level
    let adj_rows: Vec<(String, String, String, f64, i32)> = sqlx::query_as(
        "SELECT \
            CASE \
                WHEN a.source_type = 'MANUAL' AND a.item_pk IS NULL THEN 'order_manual' \
                WHEN a.source_type = 'MANUAL' AND a.item_pk IS NOT NULL THEN 'item_manual' \
                WHEN a.source_type = 'PRICE_RULE' AND a.item_pk IS NULL THEN 'order_rule' \
                WHEN a.source_type = 'PRICE_RULE' AND a.item_pk IS NOT NULL THEN 'item_rule' \
                WHEN a.source_type = 'MEMBER_GROUP' THEN 'mg' \
                WHEN a.source_type = 'COMP' THEN 'comp' \
                ELSE a.source_type \
            END AS source_key, \
            a.direction, \
            COALESCE(a.rule_receipt_name, a.rule_name, \
                CASE \
                    WHEN a.source_type = 'MANUAL' AND a.item_pk IS NULL THEN 'order_manual' \
                    WHEN a.source_type = 'MANUAL' AND a.item_pk IS NOT NULL THEN 'item_manual' \
                    WHEN a.source_type = 'MEMBER_GROUP' THEN 'mg' \
                    WHEN a.source_type = 'COMP' THEN 'comp' \
                    ELSE a.source_type \
                END \
            ) AS display_name, \
            COALESCE(SUM(a.amount), 0.0), \
            CAST(COUNT(DISTINCT a.order_pk) AS INTEGER) \
         FROM archived_order_adjustment a \
         JOIN archived_order o ON a.order_pk = o.id \
         WHERE o.status = 'COMPLETED' AND o.is_voided = 0 \
           AND o.end_time >= ?1 AND o.end_time < ?2 \
           AND a.skipped = 0 AND a.amount > 0 \
         GROUP BY source_key, a.direction, \
            CASE WHEN a.source_type = 'PRICE_RULE' THEN a.rule_id ELSE NULL END \
         ORDER BY COALESCE(SUM(a.amount), 0.0) DESC",
    )
    .bind(start_dt)
    .bind(end_dt)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let mut discount_breakdown = Vec::new();
    let mut surcharge_breakdown = Vec::new();
    for (source_key, direction, display_name, amount, order_count) in &adj_rows {
        let entry = AdjustmentEntry {
            name: display_name.clone(),
            source: source_key.clone(),
            amount: *amount,
            order_count: *order_count,
        };
        if direction == "DISCOUNT" {
            discount_breakdown.push(entry);
        } else {
            surcharge_breakdown.push(entry);
        }
    }

    Ok(Json(StoreOverview {
        revenue,
        net_revenue: revenue - refund_amount,
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
        anulacion_count,
        anulacion_amount,
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
        discount_breakdown,
        surcharge_breakdown,
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
    .fetch_all(pool).await.map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(SalesReportResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}

// ============================================================================
// Red Flags — Grouped Summary
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ItemFlags {
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
}

#[derive(Debug, Serialize)]
pub struct OrderFlags {
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
}

#[derive(Debug, Serialize)]
pub struct PaymentFlags {
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct RedFlagsResponse {
    pub item_flags: ItemFlags,
    pub order_flags: OrderFlags,
    pub payment_flags: PaymentFlags,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

/// GET /api/statistics/red-flags
pub async fn get_red_flags(
    State(state): State<ServerState>,
    Query(query): Query<StatisticsQuery>,
) -> AppResult<Json<RedFlagsResponse>> {
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or(0);

    let (start, end) = if let (Some(from), Some(to)) = (query.from, query.to) {
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

    let pool = &state.pool;

    // Query event counts by type within time range
    let event_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT ae.event_type, COUNT(*) as cnt \
         FROM archived_order_event ae \
         JOIN archived_order o ON ae.order_pk = o.id \
         WHERE o.end_time >= ?1 AND o.end_time < ?2 \
           AND ae.event_type IN (\
               'ITEM_REMOVED', 'ITEM_COMPED', 'ITEM_UNCOMPED', 'ITEM_MODIFIED', \
               'ORDER_VOIDED', 'ORDER_DISCOUNT_APPLIED', 'ORDER_SURCHARGE_APPLIED', \
               'RULE_SKIP_TOGGLED', 'PAYMENT_CANCELLED') \
         GROUP BY ae.event_type",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let mut item_flags = ItemFlags {
        removals: 0,
        comps: 0,
        uncomps: 0,
        price_modifications: 0,
    };
    let mut order_flags = OrderFlags {
        voids: 0,
        discounts: 0,
        surcharges: 0,
        rule_skips: 0,
    };
    let mut payment_flags = PaymentFlags {
        cancellations: 0,
        refund_count: 0,
        refund_amount: 0.0,
    };

    for (event_type, count) in &event_rows {
        match event_type.as_str() {
            "ITEM_REMOVED" => item_flags.removals = *count,
            "ITEM_COMPED" => item_flags.comps = *count,
            "ITEM_UNCOMPED" => item_flags.uncomps = *count,
            "ITEM_MODIFIED" => item_flags.price_modifications = *count,
            "ORDER_VOIDED" => order_flags.voids = *count,
            "ORDER_DISCOUNT_APPLIED" => order_flags.discounts = *count,
            "ORDER_SURCHARGE_APPLIED" => order_flags.surcharges = *count,
            "RULE_SKIP_TOGGLED" => order_flags.rule_skips = *count,
            "PAYMENT_CANCELLED" => payment_flags.cancellations = *count,
            _ => {}
        }
    }

    // Query refunds from credit_note
    let refund_rows: Vec<(i64, String, i64, f64)> = sqlx::query_as(
        "SELECT COALESCE(operator_id, 0), COALESCE(operator_name, ''), \
                COUNT(*), COALESCE(SUM(total_credit), 0.0) \
         FROM credit_note \
         WHERE created_at >= ?1 AND created_at < ?2 \
         GROUP BY operator_id, operator_name",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    // Aggregate refund totals for summary
    for (_, _, cnt, amt) in &refund_rows {
        payment_flags.refund_count += cnt;
        payment_flags.refund_amount += amt;
    }

    // Operator breakdown from events
    let operator_event_rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        "SELECT COALESCE(ae.operator_id, 0), COALESCE(ae.operator_name, ''), \
                ae.event_type, COUNT(*) as cnt \
         FROM archived_order_event ae \
         JOIN archived_order o ON ae.order_pk = o.id \
         WHERE o.end_time >= ?1 AND o.end_time < ?2 \
           AND ae.event_type IN (\
               'ITEM_REMOVED', 'ITEM_COMPED', 'ITEM_UNCOMPED', 'ITEM_MODIFIED', \
               'ORDER_VOIDED', 'ORDER_DISCOUNT_APPLIED', 'ORDER_SURCHARGE_APPLIED', \
               'RULE_SKIP_TOGGLED', 'PAYMENT_CANCELLED') \
         GROUP BY ae.operator_id, ae.operator_name, ae.event_type",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let mut op_map: std::collections::HashMap<i64, OperatorRedFlags> =
        std::collections::HashMap::new();

    for (op_id, op_name, event_type, count) in operator_event_rows {
        let entry = op_map.entry(op_id).or_insert_with(|| OperatorRedFlags {
            operator_id: op_id,
            operator_name: op_name.clone(),
            removals: 0,
            comps: 0,
            uncomps: 0,
            price_modifications: 0,
            voids: 0,
            discounts: 0,
            surcharges: 0,
            rule_skips: 0,
            cancellations: 0,
            refund_count: 0,
            refund_amount: 0.0,
            total_flags: 0,
        });
        match event_type.as_str() {
            "ITEM_REMOVED" => entry.removals = count,
            "ITEM_COMPED" => entry.comps = count,
            "ITEM_UNCOMPED" => entry.uncomps = count,
            "ITEM_MODIFIED" => entry.price_modifications = count,
            "ORDER_VOIDED" => entry.voids = count,
            "ORDER_DISCOUNT_APPLIED" => entry.discounts = count,
            "ORDER_SURCHARGE_APPLIED" => entry.surcharges = count,
            "RULE_SKIP_TOGGLED" => entry.rule_skips = count,
            "PAYMENT_CANCELLED" => entry.cancellations = count,
            _ => {}
        }
    }

    // Merge refund data into operator map
    for (op_id, op_name, cnt, amt) in refund_rows {
        let entry = op_map.entry(op_id).or_insert_with(|| OperatorRedFlags {
            operator_id: op_id,
            operator_name: op_name.clone(),
            removals: 0,
            comps: 0,
            uncomps: 0,
            price_modifications: 0,
            voids: 0,
            discounts: 0,
            surcharges: 0,
            rule_skips: 0,
            cancellations: 0,
            refund_count: 0,
            refund_amount: 0.0,
            total_flags: 0,
        });
        entry.refund_count = cnt;
        entry.refund_amount = amt;
    }

    // Calculate total_flags and sort
    let mut operator_breakdown: Vec<OperatorRedFlags> = op_map
        .into_values()
        .map(|mut op| {
            op.total_flags = op.removals
                + op.comps
                + op.uncomps
                + op.price_modifications
                + op.voids
                + op.discounts
                + op.surcharges
                + op.rule_skips
                + op.cancellations
                + op.refund_count;
            op
        })
        .collect();
    operator_breakdown.sort_by(|a, b| b.total_flags.cmp(&a.total_flags));

    Ok(Json(RedFlagsResponse {
        item_flags,
        order_flags,
        payment_flags,
        operator_breakdown,
    }))
}

// ============================================================================
// Red Flags — Event Log
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RedFlagLogQuery {
    pub from: i64,
    pub to: i64,
    pub event_type: Option<String>,
    pub operator_id: Option<i64>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(rename = "perPage", default = "default_log_per_page")]
    pub per_page: i32,
}

fn default_log_per_page() -> i32 {
    50
}

#[derive(Debug, Serialize)]
pub struct RedFlagLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub operator_id: i64,
    pub operator_name: String,
    pub receipt_number: String,
    pub order_id: i64,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RedFlagLogResponse {
    pub entries: Vec<RedFlagLogEntry>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

/// GET /api/statistics/red-flags/log
pub async fn get_red_flag_log(
    State(state): State<ServerState>,
    Query(query): Query<RedFlagLogQuery>,
) -> AppResult<Json<RedFlagLogResponse>> {
    let per_page = query.per_page.clamp(1, 100);
    let offset = (query.page.max(1) - 1) * per_page;
    let pool = &state.pool;

    let event_filter = query.event_type.as_deref().unwrap_or("");
    let mut entries: Vec<RedFlagLogEntry> = Vec::new();

    // 1. Order events (skip if only REFUND requested)
    if event_filter != "REFUND" {
        let mut sql = String::from(
            "SELECT ae.timestamp, ae.event_type, \
                    COALESCE(ae.operator_id, 0), COALESCE(ae.operator_name, ''), \
                    COALESCE(o.receipt_number, ''), o.order_id, ae.data \
             FROM archived_order_event ae \
             JOIN archived_order o ON ae.order_pk = o.id \
             WHERE o.end_time >= ?1 AND o.end_time < ?2 \
               AND ae.event_type IN (\
                   'ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED',\
                   'ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED',\
                   'RULE_SKIP_TOGGLED','PAYMENT_CANCELLED')",
        );
        if !event_filter.is_empty() {
            sql.push_str(&format!(" AND ae.event_type = '{event_filter}'"));
        }
        if let Some(op_id) = query.operator_id {
            sql.push_str(&format!(" AND ae.operator_id = {op_id}"));
        }

        #[allow(clippy::type_complexity)]
        let rows: Vec<(i64, String, i64, String, String, i64, Option<String>)> =
            sqlx::query_as(&sql)
                .bind(query.from)
                .bind(query.to)
                .fetch_all(pool)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;

        for (ts, etype, op_id, op_name, receipt, order_id, data) in rows {
            entries.push(RedFlagLogEntry {
                timestamp: ts,
                event_type: etype,
                operator_id: op_id,
                operator_name: op_name,
                receipt_number: receipt,
                order_id,
                detail: data,
            });
        }
    }

    // 2. Refunds (unless filtering by a specific non-REFUND event type)
    if event_filter.is_empty() || event_filter == "REFUND" {
        let mut refund_sql = String::from(
            "SELECT cn.created_at, cn.operator_id, cn.operator_name, \
                    COALESCE(o.receipt_number, ''), o.order_id, \
                    cn.total_credit, cn.reason \
             FROM credit_note cn \
             JOIN archived_order o ON cn.original_order_pk = o.id \
             WHERE cn.created_at >= ?1 AND cn.created_at < ?2",
        );
        if let Some(op_id) = query.operator_id {
            refund_sql.push_str(&format!(" AND cn.operator_id = {op_id}"));
        }

        let refund_rows: Vec<(i64, i64, String, String, i64, f64, String)> =
            sqlx::query_as(&refund_sql)
                .bind(query.from)
                .bind(query.to)
                .fetch_all(pool)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;

        for (ts, op_id, op_name, receipt, order_id, amount, reason) in refund_rows {
            entries.push(RedFlagLogEntry {
                timestamp: ts,
                event_type: "REFUND".to_string(),
                operator_id: op_id,
                operator_name: op_name,
                receipt_number: receipt,
                order_id,
                detail: Some(format!("{:.2} - {}", amount, reason)),
            });
        }
    }

    // Sort by timestamp DESC, then paginate
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let total = entries.len() as i64;
    let paginated: Vec<RedFlagLogEntry> = entries
        .into_iter()
        .skip(offset as usize)
        .take(per_page as usize)
        .collect();

    Ok(Json(RedFlagLogResponse {
        entries: paginated,
        total,
        page: query.page.max(1),
        per_page,
    }))
}

// ============================================================================
// Invoice List
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct InvoiceListQuery {
    /// Unix millis start
    pub from: Option<i64>,
    /// Unix millis end
    pub to: Option<i64>,
    #[serde(rename = "timeRange")]
    pub time_range: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub tipo: Option<String>,
    pub aeat_status: Option<String>,
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_page_size")]
    pub page_size: i32,
}

fn default_page_size() -> i32 {
    20
}

#[derive(Debug, Serialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<invoice::InvoiceListRow>,
    pub total: i64,
    pub page: i32,
    pub page_size: i32,
}

/// GET /api/statistics/invoices
pub async fn list_invoices(
    State(state): State<ServerState>,
    Query(query): Query<InvoiceListQuery>,
) -> AppResult<Json<InvoiceListResponse>> {
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or(0);

    let (start, end) = if let (Some(from), Some(to)) = (query.from, query.to) {
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

    let offset = (query.page - 1) * query.page_size;
    let (invoices, total) = invoice::list_paginated(
        &state.pool,
        start,
        end,
        query.tipo.as_deref(),
        query.aeat_status.as_deref(),
        query.page_size,
        offset,
    )
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(InvoiceListResponse {
        invoices,
        total,
        page: query.page,
        page_size: query.page_size,
    }))
}
