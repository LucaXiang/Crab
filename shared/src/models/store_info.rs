//! Store Info Model

use serde::{Deserialize, Serialize};

/// Store information entity (singleton per tenant)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreInfo {
    pub id: Option<String>,
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
    /// 营业日分界时间 (HH:MM 格式，如 "06:00")
    #[serde(default = "default_cutoff")]
    pub business_day_cutoff: String,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

fn default_cutoff() -> String {
    "02:00".to_string()
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
    /// 营业日分界时间 (HH:MM 格式)
    pub business_day_cutoff: Option<String>,
}
