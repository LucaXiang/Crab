//! Store Info Model (Singleton)
//!
//! 店铺信息，每个租户只有一条记录

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// Store info entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreInfo {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    /// 店铺名称
    pub name: String,
    /// 店铺地址
    pub address: String,
    /// 税号 / 营业执照
    pub nif: String,
    /// Logo URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    /// 联系电话
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// 电子邮箱
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 官方网站
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// 创建时间
    pub created_at: Option<String>,
    /// 更新时间
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nif: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}
