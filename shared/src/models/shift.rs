//! Shift Model (班次管理)

use serde::{Deserialize, Serialize};

/// Shift status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShiftStatus {
    #[serde(rename = "OPEN")]
    Open,
    #[serde(rename = "CLOSED")]
    Closed,
}

impl Default for ShiftStatus {
    fn default() -> Self {
        Self::Open
    }
}

/// Shift record - represents an operator's work shift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    pub id: Option<String>,
    /// Operator employee ID
    pub operator_id: String,
    /// Operator display name
    pub operator_name: String,
    /// Shift status
    pub status: ShiftStatus,
    /// Shift start time (ISO 8601)
    pub start_time: String,
    /// Shift end time (ISO 8601), null if still open
    pub end_time: Option<String>,
    /// Starting cash amount
    pub starting_cash: f64,
    /// Expected cash amount (starting + cash payments received)
    pub expected_cash: f64,
    /// Actual cash counted at close
    pub actual_cash: Option<f64>,
    /// Cash variance (actual - expected)
    pub cash_variance: Option<f64>,
    /// Whether shift was closed abnormally (power failure, etc.)
    #[serde(default)]
    pub abnormal_close: bool,
    /// Last heartbeat timestamp
    pub last_active_at: Option<String>,
    /// Notes
    pub note: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create shift payload (open shift)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftCreate {
    /// Operator employee ID
    pub operator_id: String,
    /// Operator display name
    pub operator_name: String,
    /// Starting cash amount (default 0)
    #[serde(default)]
    pub starting_cash: f64,
    /// Notes
    pub note: Option<String>,
}

/// Close shift payload (normal close with cash counting)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftClose {
    /// Actual cash counted
    pub actual_cash: f64,
    /// Notes
    pub note: Option<String>,
}

/// Force close shift payload (abnormal close without cash counting)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShiftForceClose {
    /// Notes
    pub note: Option<String>,
}

/// Update shift payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftUpdate {
    /// Update starting cash (only when OPEN)
    pub starting_cash: Option<f64>,
    /// Notes
    pub note: Option<String>,
}
