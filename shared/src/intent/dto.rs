//! Data Transfer Objects (DTOs)
//!
//! 这些类型用于前后端通信，使用 String 表示 ID (格式: "table:id")。

use serde::{Deserialize, Serialize};

// =============================================================================
// Tag (标签)
// =============================================================================

/// 创建标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
}

/// 更新标签
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// =============================================================================
// Category (分类)
// =============================================================================

/// 创建分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    /// 厨房打印机 ID (字符串形式)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_printer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_kitchen_print_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_label_print_enabled: Option<bool>,
}

/// 更新分类
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_printer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_kitchen_print_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_label_print_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// =============================================================================
// Product (商品)
// =============================================================================

/// 创建商品
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDto {
    pub name: String,
    /// 分类 ID
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_multi_spec: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_print_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_printer: Option<String>,
    /// -1: 继承分类设置, 0: 禁用, 1: 启用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_kitchen_print_enabled: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_label_print_enabled: Option<i32>,
}

/// 更新商品
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_multi_spec: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_print_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_printer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_kitchen_print_enabled: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_label_print_enabled: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// =============================================================================
// ProductSpecification (商品规格)
// =============================================================================

/// 创建商品规格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecDto {
    /// 商品 ID
    pub product: String,
    pub name: String,
    /// 价格 (分)
    pub price: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_root: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    /// 标签 ID 列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// 更新商品规格
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductSpecUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_root: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// =============================================================================
// Attribute (属性)
// =============================================================================

/// 属性选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOptionDto {
    pub name: String,
    /// 价格调整 (分)
    #[serde(default)]
    pub price: i32,
    #[serde(default)]
    pub is_default: bool,
}

/// 创建属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDto {
    pub name: String,
    /// 属性类型: "single_select", "multi_select", "text"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_receipt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_printer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_global: Option<bool>,
    /// 内嵌选项列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<AttributeOptionDto>>,
}

/// 更新属性
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttributeUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attr_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_receipt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_printer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_global: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<AttributeOptionDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// =============================================================================
// Zone (区域)
// =============================================================================

/// 创建区域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 更新区域
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// =============================================================================
// DiningTable (桌台)
// =============================================================================

/// 创建桌台
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableDto {
    pub name: String,
    /// 区域 ID
    pub zone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i32>,
}

/// 更新桌台
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiningTableUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

// =============================================================================
// PriceRule (价格规则)
// =============================================================================

/// 创建价格规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRuleDto {
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// "DISCOUNT" | "SURCHARGE"
    pub rule_type: String,
    /// "GLOBAL" | "CATEGORY" | "TAG" | "PRODUCT"
    pub product_scope: String,
    /// 目标 ID (根据 scope)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// 区域范围: "zone:all", "zone:retail", 或 "zone:xxx"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_scope: Option<String>,
    /// "PERCENTAGE" | "FIXED_AMOUNT"
    pub adjustment_type: String,
    /// 调整值 (百分比如30=30%, 固定金额单位:分)
    pub adjustment_value: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_stackable: Option<bool>,
    /// "ALWAYS" | "SCHEDULE" | "ONETIME"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_config: Option<ScheduleConfigDto>,
}

/// 周期配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduleConfigDto {
    /// 星期几 (0=周日, 1=周一, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_of_week: Option<Vec<i32>>,
    /// 开始时间 (HH:MM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// 结束时间 (HH:MM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
}

/// 更新价格规则
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PriceRuleUpdateDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// 区域范围: "zone:all", "zone:retail", 或 "zone:xxx"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone_scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adjustment_value: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_stackable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_config: Option<ScheduleConfigDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
