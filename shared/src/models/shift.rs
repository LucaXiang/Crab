//! Shift Model (班次管理)

use serde::{Deserialize, Serialize};

/// Shift status
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum ShiftStatus {
    #[default]
    Open,
    Closed,
}

/// Shift record - represents an operator's work shift
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Shift {
    pub id: i64,
    pub operator_id: i64,
    pub operator_name: String,
    pub status: ShiftStatus,
    /// Shift start time (Unix timestamp millis)
    pub start_time: i64,
    /// Shift end time (Unix timestamp millis), null if still open
    pub end_time: Option<i64>,
    pub starting_cash: f64,
    /// Expected cash amount (starting + cash payments received)
    pub expected_cash: f64,
    /// Actual cash counted at close
    pub actual_cash: Option<f64>,
    /// Cash variance (actual - expected)
    pub cash_variance: Option<f64>,
    /// Whether shift was closed abnormally
    pub abnormal_close: bool,
    /// Last heartbeat timestamp (Unix timestamp millis)
    pub last_active_at: Option<i64>,
    pub note: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Create shift payload (open shift)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftCreate {
    pub operator_id: i64,
    pub operator_name: String,
    #[serde(default)]
    pub starting_cash: f64,
    pub note: Option<String>,
}

/// Close shift payload (normal close with cash counting)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftClose {
    pub actual_cash: f64,
    pub note: Option<String>,
}

/// Force close shift payload (abnormal close without cash counting)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShiftForceClose {
    pub note: Option<String>,
}

/// Update shift payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftUpdate {
    pub starting_cash: Option<f64>,
    pub note: Option<String>,
}
