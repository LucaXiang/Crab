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
    /// ISO 4217 货币代码 (e.g. "EUR", "USD", "CNY")
    pub currency_code: Option<String>,
    /// 货币符号 (e.g. "€", "$", "¥")
    pub currency_symbol: Option<String>,
    /// 货币小数位数 (e.g. 2 for EUR, 0 for JPY)
    pub currency_decimal_places: Option<i32>,
    /// IANA 时区 (e.g. "Europe/Madrid", "Asia/Shanghai")
    pub timezone: Option<String>,
    /// 收据 locale (e.g. "es-ES", "zh-CN", "en")
    pub receipt_locale: Option<String>,
    /// 收据页眉自定义文本
    pub receipt_header: Option<String>,
    /// 收据页脚自定义文本
    pub receipt_footer: Option<String>,
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
    pub currency_code: Option<String>,
    pub currency_symbol: Option<String>,
    pub currency_decimal_places: Option<i32>,
    pub timezone: Option<String>,
    pub receipt_locale: Option<String>,
    pub receipt_header: Option<String>,
    pub receipt_footer: Option<String>,
}
