//! 打印相关数据结构
//!
//! 定义收据、标签打印所需的数据结构

use serde::{Deserialize, Serialize};

/// 店铺信息 (用于收据头部)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreInfo {
    pub name: String,
    pub address: String,
    pub nif: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub logo_url: Option<String>,
}

/// 附加费信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurchargeInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub value: f64,
    pub amount: f64,
}

/// 选项信息 (用于订单项)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedOption {
    pub attribute_name: String,
    pub option_name: String,
    pub receipt_name: Option<String>,
    pub price_modifier: f64,
}

/// 订单项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptItem {
    pub name: String,
    pub quantity: i32,
    pub price: f64,
    pub total: f64,
    pub tax_rate: Option<f64>,
    pub discount_percent: Option<f64>,
    pub original_price: Option<f64>,
    pub selected_options: Option<Vec<SelectedOption>>,
    pub spec_name: Option<String>,
}

/// 收据数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptData {
    pub order_id: String,
    pub timestamp: String,
    pub table_name: String,
    pub zone_name: Option<String>,
    pub guest_count: Option<i32>,
    pub opened_at: Option<String>,
    pub checkout_time: Option<String>,
    pub void_reason: Option<String>,
    pub reprint: bool,
    #[serde(default)]
    pub pre_payment: bool,
    pub store_info: Option<StoreInfo>,
    pub surcharge: Option<SurchargeInfo>,
    pub items: Vec<ReceiptItem>,
    pub total_amount: f64,
    pub qr_data: Option<String>,
}

/// 标签数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelData {
    pub product_name: String,
    pub specification: Option<String>,
    pub price: Option<f64>,
    pub barcode: Option<String>,
    pub custom_fields: Option<serde_json::Value>,
}
