//! Store Info Model (Singleton)
//!
//! 店铺信息，每个租户只有一条记录

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// Store info entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
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
    /// 营业日分界时间 (HH:MM 格式，如 "06:00")
    /// 用于跨天班次判断和日结报告计算
    /// 默认 "02:00" (凌晨两点)，酒吧/夜店可设置为 "06:00"
    #[serde(default = "default_business_day_cutoff", deserialize_with = "deserialize_business_day_cutoff")]
    pub business_day_cutoff: String,
    /// 创建时间
    pub created_at: Option<i64>,
    /// 更新时间
    pub updated_at: Option<i64>,
}

fn default_business_day_cutoff() -> String {
    "02:00".to_string()
}

fn deserialize_business_day_cutoff<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(default_business_day_cutoff())
    } else {
        Ok(s)
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
    /// 营业日分界时间 (HH:MM 格式)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_day_cutoff: Option<String>,
}
