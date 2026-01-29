//! Shift Model (班次管理)

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

pub type ShiftId = RecordId;

/// Shift status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShiftStatus {
    Open,
    Closed,
}

impl Default for ShiftStatus {
    fn default() -> Self {
        Self::Open
    }
}

/// Shift entity (班次)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<ShiftId>,

    /// 操作员 ID
    #[serde(with = "serde_helpers::record_id")]
    pub operator_id: RecordId,

    /// 操作员姓名快照
    pub operator_name: String,

    /// 班次状态
    #[serde(default)]
    pub status: ShiftStatus,

    /// 开班时间 (Unix timestamp millis)
    pub start_time: i64,

    /// 收班时间 (Unix timestamp millis)
    pub end_time: Option<i64>,

    /// 备用金 (开班时的现金准备金)
    #[serde(default)]
    pub starting_cash: f64,

    /// 预期现金 (starting_cash + 班次内现金收款)
    #[serde(default)]
    pub expected_cash: f64,

    /// 实际现金 (收班时盘点)
    pub actual_cash: Option<f64>,

    /// 现金差异 (actual_cash - expected_cash)
    pub cash_variance: Option<f64>,

    /// 是否异常关闭 (断电/崩溃)
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub abnormal_close: bool,

    /// 最后活动时间 (心跳)
    pub last_active_at: Option<i64>,

    /// 备注
    pub note: Option<String>,

    /// 创建时间
    pub created_at: Option<i64>,

    /// 更新时间
    pub updated_at: Option<i64>,
}

/// Create shift payload (开班)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftCreate {
    pub operator_id: String,
    pub operator_name: String,
    #[serde(default)]
    pub starting_cash: f64,
    pub note: Option<String>,
}

/// Close shift payload (收班)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftClose {
    pub actual_cash: f64,
    pub note: Option<String>,
}

/// Force close shift payload (强制关闭)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftForceClose {
    pub note: Option<String>,
}

/// Update shift payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starting_cash: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Shift summary for list view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftSummary {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<ShiftId>,
    pub operator_name: String,
    pub status: ShiftStatus,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub starting_cash: f64,
    pub expected_cash: f64,
    pub actual_cash: Option<f64>,
    pub cash_variance: Option<f64>,
    pub abnormal_close: bool,
    pub total_orders: i32,
    pub total_sales: f64,
}
