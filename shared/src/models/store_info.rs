//! Store Info Model

use serde::{Deserialize, Serialize};

/// Store information entity (singleton per tenant)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StoreInfo {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub address: String,
    /// Tax identification number (NIF)
    #[serde(default)]
    pub nif: String,
    pub logo_url: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    /// 营业日分界时间 — 从午夜 00:00 起的偏移分钟数 (0-480，即 00:00-08:00)
    #[serde(default)]
    pub business_day_cutoff: i32,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// Update store info payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoreInfoUpdate {
    pub name: Option<String>,
    pub address: Option<String>,
    pub nif: Option<String>,
    pub logo_url: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub business_day_cutoff: Option<i32>,
}
