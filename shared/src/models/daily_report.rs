//! Daily Report Model (日结报告)

use serde::{Deserialize, Serialize};

/// Tax breakdown by rate (independent table)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct TaxBreakdown {
    pub id: i64,
    pub report_id: i64,
    /// Tax rate (0, 4, 10, 21)
    pub tax_rate: i32,
    /// Net amount (before tax)
    pub net_amount: f64,
    /// Tax amount
    pub tax_amount: f64,
    /// Gross amount (after tax)
    pub gross_amount: f64,
    /// Number of orders with this tax rate
    pub order_count: i64,
}

/// Payment method breakdown (independent table)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct PaymentMethodBreakdown {
    pub id: i64,
    pub report_id: i64,
    /// Payment method name
    pub method: String,
    /// Total amount
    pub amount: f64,
    /// Number of payments
    pub count: i64,
}

/// Daily Report - end-of-day settlement report
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct DailyReport {
    pub id: i64,
    /// Business date (YYYY-MM-DD format)
    pub business_date: String,
    pub total_orders: i64,
    pub completed_orders: i64,
    pub void_orders: i64,
    pub total_sales: f64,
    pub total_paid: f64,
    pub total_unpaid: f64,
    pub void_amount: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    /// When the report was generated (Unix millis)
    pub generated_at: Option<i64>,
    /// Who generated the report (employee identifier)
    pub generated_by_id: Option<i64>,
    pub generated_by_name: Option<String>,
    pub note: Option<String>,

    // -- Relations (populated by application code, skipped by FromRow) --
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub tax_breakdowns: Vec<TaxBreakdown>,
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub payment_breakdowns: Vec<PaymentMethodBreakdown>,
}

/// Generate daily report payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReportGenerate {
    /// Business date to generate report for (YYYY-MM-DD)
    pub business_date: String,
    pub note: Option<String>,
}
