//! Daily Report Model (日结报告)

use serde::{Deserialize, Serialize};

/// Tax breakdown by rate (Spain: 0%, 4%, 10%, 21%)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxBreakdown {
    /// Tax rate (0, 4, 10, 21)
    pub tax_rate: i32,
    /// Net amount (before tax)
    pub net_amount: f64,
    /// Tax amount
    pub tax_amount: f64,
    /// Gross amount (after tax)
    pub gross_amount: f64,
    /// Number of orders with this tax rate
    pub order_count: i32,
}

/// Payment method breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodBreakdown {
    /// Payment method name
    pub method: String,
    /// Total amount
    pub amount: f64,
    /// Number of payments
    pub count: i32,
}

/// Daily Report - end-of-day settlement report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub id: Option<String>,
    /// Business date (YYYY-MM-DD format)
    pub business_date: String,
    /// Total number of orders
    pub total_orders: i32,
    /// Completed orders count
    pub completed_orders: i32,
    /// Voided orders count
    pub void_orders: i32,
    /// Total sales amount
    pub total_sales: f64,
    /// Total paid amount
    pub total_paid: f64,
    /// Total unpaid amount
    pub total_unpaid: f64,
    /// Voided order total amount
    pub void_amount: f64,
    /// Total tax collected
    #[serde(default)]
    pub total_tax: f64,
    /// Total discount applied
    #[serde(default)]
    pub total_discount: f64,
    /// Total surcharge applied
    #[serde(default)]
    pub total_surcharge: f64,
    /// Tax breakdown by rate
    #[serde(default)]
    pub tax_breakdowns: Vec<TaxBreakdown>,
    /// Payment breakdown by method
    #[serde(default)]
    pub payment_breakdowns: Vec<PaymentMethodBreakdown>,
    /// When the report was generated (ISO 8601)
    pub generated_at: String,
    /// Who generated the report (employee ID)
    pub generated_by_id: Option<String>,
    /// Who generated the report (name)
    pub generated_by_name: Option<String>,
    /// Notes
    pub note: Option<String>,
}

/// Generate daily report payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReportGenerate {
    /// Business date to generate report for (YYYY-MM-DD)
    pub business_date: String,
    /// Notes
    pub note: Option<String>,
}
