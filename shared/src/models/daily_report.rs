//! Daily Report Model (日结报告)

use serde::{Deserialize, Serialize};

/// Shift breakdown within a daily report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct ShiftBreakdown {
    pub id: i64,
    pub report_id: i64,
    pub shift_id: i64,
    pub operator_id: i64,
    pub operator_name: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub starting_cash: f64,
    pub expected_cash: f64,
    pub actual_cash: Option<f64>,
    pub cash_variance: Option<f64>,
    pub abnormal_close: bool,
    pub total_orders: i64,
    pub completed_orders: i64,
    pub void_orders: i64,
    pub total_sales: f64,
    pub total_paid: f64,
    pub void_amount: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
}

/// Daily Report - shift settlement record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct DailyReport {
    pub id: i64,
    /// Business date (YYYY-MM-DD format)
    pub business_date: String,
    /// Net revenue (total_sales - refund_amount)
    pub net_revenue: f64,
    /// Total completed orders
    pub total_orders: i64,
    /// Total refund amount from credit notes
    pub refund_amount: f64,
    /// Number of credit notes issued
    pub refund_count: i64,
    /// Whether this report was auto-generated (e.g. by shift close)
    pub auto_generated: bool,
    /// When the report was generated (Unix millis)
    pub generated_at: Option<i64>,
    /// Who generated the report (employee identifier)
    pub generated_by_id: Option<i64>,
    pub generated_by_name: Option<String>,
    pub note: Option<String>,

    // -- Relations (populated by application code, skipped by FromRow) --
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub shift_breakdowns: Vec<ShiftBreakdown>,
}

/// Generate daily report payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReportGenerate {
    /// Business date to generate report for (YYYY-MM-DD)
    pub business_date: String,
    pub note: Option<String>,
}
