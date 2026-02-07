//! System Issue Model

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// System issue record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct SystemIssue {
    pub id: i64,
    pub source: String,
    pub kind: String,
    pub blocking: bool,
    pub target: Option<String>,
    /// JSON object (flexible schema)
    #[cfg_attr(feature = "db", sqlx(json))]
    pub params: HashMap<String, String>,
    pub title: Option<String>,
    pub description: Option<String>,
    /// JSON array of option strings
    #[cfg_attr(feature = "db", sqlx(json))]
    pub options: Vec<String>,
    pub status: String,
    pub response: Option<String>,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<i64>,
    pub created_at: i64,
}

/// Create system issue payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIssueCreate {
    pub source: String,
    pub kind: String,
    pub blocking: bool,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

/// Resolve system issue payload
#[derive(Debug, Clone, Deserialize)]
pub struct SystemIssueResolve {
    pub id: i64,
    pub response: String,
}
