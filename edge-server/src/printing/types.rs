//! Kitchen/Label printing types

use serde::{Deserialize, Serialize};

/// 打印上下文 (完整 JSON，模板自取所需字段)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintItemContext {
    // 分类
    pub category_id: String,
    pub category_name: String,

    // 商品
    pub product_id: String,
    pub external_id: Option<i64>,        // 商品编号 (root spec)
    pub kitchen_name: String,            // 厨房打印名称
    pub product_name: String,            // 原始商品名

    // 规格
    pub spec_name: Option<String>,

    // 数量
    pub quantity: i32,
    pub index: Option<String>,           // 标签用："2/5"

    // 属性/做法
    pub options: Vec<String>,

    // 备注
    pub note: Option<String>,

    // 打印目的地
    pub kitchen_destinations: Vec<String>,
    pub label_destinations: Vec<String>,
}

/// 厨房订单菜品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenOrderItem {
    pub context: PrintItemContext,
}

/// 一次点单的厨房记录（对应一个 ItemsAdded 事件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenOrder {
    pub id: String,                      // = event_id
    pub order_id: String,
    pub table_name: Option<String>,
    pub created_at: i64,                 // 时间戳
    pub items: Vec<KitchenOrderItem>,
    pub print_count: u32,                // 打印次数
}

/// 标签打印记录（单品级别）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelPrintRecord {
    pub id: String,                      // UUID
    pub order_id: String,
    pub kitchen_order_id: String,        // 关联的 KitchenOrder
    pub table_name: Option<String>,
    pub created_at: i64,
    pub context: PrintItemContext,
    pub print_count: u32,
}

/// 商品打印配置（内存缓存）
#[derive(Debug, Clone)]
pub struct ProductPrintConfig {
    pub product_id: String,
    pub product_name: String,
    pub kitchen_name: String,
    pub kitchen_print_destinations: Vec<String>,
    pub label_print_destinations: Vec<String>,
    pub is_kitchen_print_enabled: i32,   // -1=继承, 0=禁用, 1=启用
    pub is_label_print_enabled: i32,     // -1=继承, 0=禁用, 1=启用
    pub root_spec_external_id: Option<i64>,
    pub category_id: String,
}

/// 分类打印配置（内存缓存）
#[derive(Debug, Clone)]
pub struct CategoryPrintConfig {
    pub category_id: String,
    pub category_name: String,
    pub kitchen_print_destinations: Vec<String>,
    pub label_print_destinations: Vec<String>,
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
}
