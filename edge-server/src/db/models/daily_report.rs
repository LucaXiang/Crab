//! Daily Report Model (日结报告)

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

pub type DailyReportId = RecordId;

/// Tax rate breakdown (西班牙税率分类)
/// Spain IVA rates: 0%, 4%, 10%, 21%
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxBreakdown {
    /// 税率 (0, 4, 10, 21)
    pub tax_rate: i32,

    /// 净额 (不含税)
    #[serde(default)]
    pub net_amount: f64,

    /// 税额
    #[serde(default)]
    pub tax_amount: f64,

    /// 总额 (含税)
    #[serde(default)]
    pub gross_amount: f64,

    /// 订单数
    #[serde(default)]
    pub order_count: i32,
}

/// Payment method breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodBreakdown {
    /// 支付方式
    pub method: String,

    /// 总金额
    #[serde(default)]
    pub amount: f64,

    /// 笔数
    #[serde(default)]
    pub count: i32,
}

/// Daily report entity (日结报告)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<DailyReportId>,

    /// 营业日期 (YYYY-MM-DD)
    pub business_date: String,

    // === 订单统计 ===
    /// 总订单数
    #[serde(default)]
    pub total_orders: i32,

    /// 完成订单数
    #[serde(default)]
    pub completed_orders: i32,

    /// 作废订单数
    #[serde(default)]
    pub void_orders: i32,

    // === 金额统计 ===
    /// 总营业额 (completed orders)
    #[serde(default)]
    pub total_sales: f64,

    /// 已支付金额
    #[serde(default)]
    pub total_paid: f64,

    /// 未支付金额
    #[serde(default)]
    pub total_unpaid: f64,

    /// 作废订单金额
    #[serde(default)]
    pub void_amount: f64,

    /// 税额总计
    #[serde(default)]
    pub total_tax: f64,

    /// 折扣总计
    #[serde(default)]
    pub total_discount: f64,

    /// 附加费总计
    #[serde(default)]
    pub total_surcharge: f64,

    // === 税率分类 (西班牙) ===
    #[serde(default)]
    pub tax_breakdowns: Vec<TaxBreakdown>,

    // === 支付方式分类 ===
    #[serde(default)]
    pub payment_breakdowns: Vec<PaymentMethodBreakdown>,

    /// 生成时间
    pub generated_at: Option<String>,

    /// 生成人 ID
    pub generated_by_id: Option<String>,

    /// 生成人姓名
    pub generated_by_name: Option<String>,

    /// 备注
    pub note: Option<String>,
}

/// Generate daily report request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReportGenerate {
    pub business_date: String,
    pub note: Option<String>,
}
