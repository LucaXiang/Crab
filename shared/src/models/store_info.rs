//! Store Info Model

use serde::{Deserialize, Serialize};

/// Store information entity (singleton per tenant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreInfo {
    pub id: Option<String>,
    pub name: String,
    pub address: String,
    /// Tax identification number (NIF)
    pub nif: String,
    pub logo_url: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Default for StoreInfo {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            address: String::new(),
            nif: String::new(),
            logo_url: None,
            phone: None,
            email: None,
            website: None,
            created_at: None,
            updated_at: None,
        }
    }
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
