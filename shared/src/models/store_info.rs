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
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
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
}
