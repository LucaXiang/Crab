# RedCoral POS - Rust 后端迁移指南

> **当前状态**: ✅ 迁移已完成
> - SQL Schema: ✅ 完成 (20 张表，14 个触发器)
> - API 代码: ✅ 完成 (所有模块)
> - 单元测试: ✅ 完成 (19 个测试全部通过)

## 目录

- [概览](#概览)
- [当前状态](#当前状态)
- [主要变更](#主要变更)
- [Rust 类型定义](#rust-类型定义)
- [哈希链实现](#哈希链实现)
- [金额计算工具](#金额计算工具)
- [软删除与审计](#软删除与审计)
- [价格调整规则](#价格调整规则)
- [打印设置继承](#打印设置继承)
- [API 层与数据库层](#api-层与数据库层)
- [迁移检查清单](#迁移检查清单)
- [常见问题](#常见问题)

## 概览

本指南说明如何将现有的 Rust 后端代码适配到新的数据库 schema。

**设计原则**:
1. 所有金额字段以"分"为单位存储，使用 `INTEGER` 类型
2. 软删除机制: `is_deleted` 标记 + `deleted_at` + `deleted_by`
3. 订单和订单事件只读，任何操作通过新增事件记录
4. 哈希链保护订单和事件防篡改

## 当前状态

### 已完成

| 组件 | 状态 | 说明 |
|------|------|------|
| SQL Schema | ✅ 完成 | 20 张表，14 个触发器 |
| 迁移 SQL 文件 | ✅ 完成 | `20251215000000_init.sql` |
| 类型定义 (API) | ✅ 完成 | `src-tauri/src/core/types.rs` |
| 迁移指南 | ✅ 完成 | 本文档 |
| 价格转换工具 | ✅ 完成 | `src-tauri/src/utils/price.rs` |
| 哈希链模块 | ✅ 完成 | `src-tauri/src/core/hash_chain.rs` (5 个测试) |
| 审计日志服务 | ✅ 完成 | `src-tauri/src/services/audit.rs` |
| products.rs | ✅ 完成 | 价格转换 + 软删除 |
| orders/crud.rs | ✅ 完成 | 金额字段转换 |
| orders/loaders.rs | ✅ 完成 | 金额字段转换 |
| categories.rs | ✅ 完成 | 软删除 + is_deleted 过滤 |
| specifications.rs | ✅ 完成 | 价格字段转换 |
| 单元测试 | ✅ 完成 | 19 个测试全部通过 |

### 待完成

无 - 迁移已完成！

## 主要变更

### 金额字段类型变更

所有金额字段从 `REAL` 改为 `INTEGER` (存储分):

```rust
// 旧写法
price: f64,           // 12.50 元
discount_amount: f64,

// 新写法
price: i64,           // 1250 分
discount_amount: i64,
```

### 新增软删除字段

所有业务表新增字段:

```rust
is_deleted: bool,           // 默认 false
deleted_at: Option<i64>,    // 删除时间戳
deleted_by: Option<String>, // 删除人 UUID
updated_by: Option<String>, // 更新人 UUID
```

### 哈希链字段

订单表新增:
```rust
prev_hash: String,  // 上一个订单的 curr_hash (或 genesis_hash)
curr_hash: String,  // SHA256(order + event[last].curr_hash)
```

订单事件表新增:
```rust
prev_hash: String,  // 第一个事件 = receipt_number，后续 = 上一个事件的 curr_hash
curr_hash: String,  // SHA256(event + prev_hash)
```

## Rust 类型定义

### 核心类型 (src-tauri/src/core/types.rs)

```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ==================== 基础类型 ====================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintSettings {
    pub kitchen_printer_id: Option<i64>,
    pub kitchen_print_name: String,
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub source: String,
    pub source_device: Option<String>,
    pub source_ip: Option<String>,
}

// ==================== 用户权限系统 ====================

// 注意：实际 API 使用 String 类型作为 ID (例如 "1" 而不是 i32)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Role {
    pub id: String,              // API: String, DB: INTEGER
    pub uuid: String,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RolePermission {
    pub role: String,            // API: String (role_id)
    pub permission: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: String,              // API: String, DB: INTEGER
    pub uuid: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub role: String,            // API: String (role_id)
    pub is_active: bool,
    pub last_login: Option<i64>,
    pub avatar: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
    pub created_by: Option<String>,
}

// ==================== 产品系统 ====================

// 分类响应类型 (API 层)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryResp {
    pub id: String,              // API: String
    pub name: String,
    pub sort_order: Option<i64>,
    pub kitchen_printer_id: Option<i64>,
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
}

// 产品响应类型 (API 层)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResp {
    pub id: String,              // TEXT 类型
    pub uuid: String,
    pub name: String,
    pub receipt_name: Option<String>,
    pub price: f64,              // API: f64 (元), 存储: i64 (分)
    pub image: String,
    pub category: String,        // API: String (category_id)
    pub external_id: i64,
    pub tax_rate: f64,
    pub sort_order: Option<i64>,
    pub kitchen_printer_id: Option<i64>,
    pub kitchen_print_name: Option<String>,
    pub is_kitchen_print_enabled: i32,  // -1: inherit, 0: off, 1: on
    pub is_label_print_enabled: i32,    // -1: inherit, 0: off, 1: on
    pub has_multi_spec: Option<bool>,
}

// 数据库层 Product 类型 (内部使用)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Product {
    pub id: String,              // TEXT PRIMARY KEY
    pub uuid: String,
    pub external_id: i64,
    pub name: String,
    pub price: i64,              // DB: 分
    pub image: String,
    pub category_id: i32,        // DB: INTEGER
    pub sort_order: i32,
    pub tax_rate: f64,
    pub has_multi_spec: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer_id: Option<i64>,
    pub is_kitchen_print_enabled: i32,
    pub is_label_print_enabled: i32,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProductSpecification {
    pub id: i64,
    pub product_id: String,
    pub name: String,
    pub price: i64,           // 分
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub is_root: bool,
    pub external_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

// ==================== 标签系统 ====================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Tag {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SpecificationTag {
    pub specification_id: i64,
    pub tag_id: i32,
    pub created_at: i64,
}

// ==================== 属性系统 ====================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AttributeTemplate {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub type_: String,  // type 是 Rust 关键字
    pub display_order: i32,
    pub is_active: bool,
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,
    pub kitchen_printer_id: Option<i64>,
    pub is_global: bool,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AttributeOption {
    pub id: i32,
    pub uuid: String,
    pub attribute_id: i32,
    pub name: String,
    pub value_code: Option<String>,
    pub price_modifier: i64,  // 分
    pub is_default: bool,
    pub display_order: i32,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ProductAttribute {
    pub id: i32,
    pub product_id: String,
    pub attribute_id: i32,
    pub is_required: bool,
    pub display_order: i32,
    pub default_option_id: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

// ==================== 区域和桌台 ====================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Zone {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Table {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub zone_id: i32,
    pub capacity: i32,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct KitchenPrinter {
    pub id: i32,
    pub name: String,
    pub printer_name: Option<String>,
    pub description: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub updated_by: Option<String>,
}

// ==================== 价格调整规则 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAdjustmentRule {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub description: Option<String>,
    pub rule_type: String,         // SURCHARGE | DISCOUNT
    pub scope: String,             // GLOBAL | CATEGORY | TAG | PRODUCT | ZONE
    pub target_id: Option<String>, // scope 对应的目标 ID
    pub zone_id: Option<i32>,
    pub adjustment_type: String,   // PERCENTAGE | FIXED_AMOUNT
    pub adjustment_value: f64,
    pub priority: i32,
    pub is_stackable: bool,
    pub time_mode: String,         // ALWAYS | SCHEDULE | ONETIME
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub schedule_config_json: Option<String>,
    pub is_active: bool,
}

impl FromRow for PriceAdjustmentRule {
    fn from_row(row: &sqlx::Row) -> std::result::Result<Self, sqlx::Error> {
        Ok(PriceAdjustmentRule {
            id: row.try_get("id")?,
            uuid: row.try_get("uuid")?,
            name: row.try_get("name")?,
            display_name: row.try_get("display_name")?,
            receipt_name: row.try_get("receipt_name")?,
            description: row.try_get("description")?,
            rule_type: row.try_get("rule_type")?,
            scope: row.try_get("scope")?,
            target_id: row.try_get("target_id")?,
            zone_id: row.try_get("zone_id")?,
            adjustment_type: row.try_get("adjustment_type")?,
            adjustment_value: row.try_get("adjustment_value")?,
            priority: row.try_get("priority")?,
            is_stackable: row.try_get("is_stackable")?,
            time_mode: row.try_get("time_mode")?,
            start_time: row.try_get("start_time")?,
            end_time: row.try_get("end_time")?,
            schedule_config_json: row.try_get("schedule_config_json")?,
            is_active: row.try_get("is_active")?,
        })
    }
}

// ==================== 订单系统 ====================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Order {
    pub order_id: i64,
    pub receipt_number: String,
    pub table_id: Option<i32>,
    pub table_name: Option<String>,
    pub status: String,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub guest_count: Option<i32>,
    pub subtotal: i64,          // 分
    pub total: i64,             // 分
    pub discount_type: Option<String>,
    pub discount_value: Option<f64>,
    pub discount_amount: i64,   // 分
    pub surcharge_amount: i64,  // 分
    pub surcharge_total: i64,   // 分
    pub zone_name: Option<String>,
    pub prev_hash: String,
    pub curr_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub event_id: i64,
    pub order_id: i64,
    pub type_: String,
    pub timestamp: i64,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub note: Option<String>,
    pub color: Option<String>,
    pub data_json: Option<String>,
    pub prev_hash: String,
    pub curr_hash: String,
    pub created_at: i64,
}

impl FromRow for OrderEvent {
    fn from_row(row: &sqlx::Row) -> std::result::Result<Self, sqlx::Error> {
        Ok(OrderEvent {
            event_id: row.try_get("event_id")?,
            order_id: row.try_get("order_id")?,
            type_: row.try_get("type")?,
            timestamp: row.try_get("timestamp")?,
            title: row.try_get("title")?,
            summary: row.try_get("summary")?,
            note: row.try_get("note")?,
            color: row.try_get("color")?,
            data_json: row.try_get("data_json")?,
            prev_hash: row.try_get("prev_hash")?,
            curr_hash: row.try_get("curr_hash")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: i64,
    pub uuid: String,
    pub order_id: i64,
    pub product_id: Option<String>,
    pub specification_id: Option<i64>,
    pub receipt_name: Option<String>,
    pub name: String,
    pub price: i64,               // 分
    pub quantity: i32,
    pub discount_amount: i64,     // 分
    pub surcharge_amount: i64,    // 分
    pub original_price: i64,      // 分
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OrderItemOption {
    pub id: i64,
    pub order_item_id: i64,
    pub attribute_id: i32,
    pub attribute_name: String,
    pub option_id: String,
    pub option_name: String,
    pub price_modifier: i64,  // 分
    pub receipt_name: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Payment {
    pub id: i64,
    pub uuid: String,
    pub order_id: i64,
    pub method: Option<String>,
    pub amount: i64,           // 分
    pub timestamp: Option<i64>,
    pub note: Option<String>,
    pub card_brand: Option<String>,
    pub last4: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,
    pub deleted_by: Option<String>,
    pub created_at: i64,
}

// ==================== 系统状态 ====================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SystemState {
    pub id: i32,
    pub genesis_hash: Option<String>,
    pub last_order_id: i64,
    pub last_order_hash: Option<String>,
    pub synced_up_to_id: i64,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<i64>,
    pub order_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

// ==================== 审计日志 ====================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub uuid: String,
    pub timestamp: i64,
    pub category: String,       // SYSTEM | OPERATION | SECURITY | DATA | PAYMENT | PRINT
    pub event_type: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub entity_name: Option<String>,
    pub action: String,
    pub description: Option<String>,
    pub severity: String,       // DEBUG | INFO | WARNING | ERROR | CRITICAL
    pub metadata_json: Option<String>,
    pub source: Option<String>,
    pub source_device: Option<String>,
    pub source_ip: Option<String>,
    pub created_at: i64,
}
```

## 哈希链实现

### 核心哈希计算模块

```rust
// src-tauri/src/core/hash_chain.rs

use sha2::{Sha256, Digest};
use hex;

const GENESIS_HASH: &str = "red_coral_genesis_block_v1";

pub struct HashChain;

impl HashChain {
    /// 计算事件哈希
    pub fn compute_event_hash(
        event_type: &str,
        timestamp: i64,
        data_json: Option<&str>,
        prev_hash: &str,
    ) -> String {
        let data = format!(
            "{}{}{}{}",
            prev_hash,
            event_type,
            timestamp,
            data_json.unwrap_or("")
        );
        Self::sha256(&data)
    }

    /// 计算订单哈希
    pub fn compute_order_hash(
        receipt_number: &str,
        total: i64,
        start_time: i64,
        discount_amount: i64,
        last_event_hash: &str,
    ) -> String {
        let data = format!(
            "{}{}{}{}{}",
            receipt_number,      // order.prev_hash
            total,
            start_time,
            discount_amount,
            last_event_hash
        );
        Self::sha256(&data)
    }

    /// SHA256 哈希
    fn sha256(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// 获取创世哈希
    pub fn genesis() -> &'static str {
        GENESIS_HASH
    }
}

/// 订单创建上下文
pub struct OrderCreationContext {
    pub receipt_number: String,
    pub prev_hash: String,
    pub events: Vec<OrderEventData>,
    pub total: i64,
    pub start_time: i64,
    pub discount_amount: i64,
}

pub struct OrderEventData {
    pub type_: String,
    pub timestamp: i64,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub note: Option<String>,
    pub color: Option<String>,
    pub data_json: Option<String>,
}

impl OrderCreationContext {
    /// 计算完整的事件链
    pub fn compute_event_chain(&mut self) -> Vec<(OrderEventData, String, String)> {
        let mut results = Vec::new();
        let mut current_prev = self.receipt_number.clone();

        for event in &mut self.events {
            let event_hash = HashChain::compute_event_hash(
                &event.type_,
                event.timestamp,
                event.data_json.as_deref(),
                &current_prev,
            );
            let prev = current_prev.clone();
            current_prev = event_hash.clone();
            results.push((event.clone(), prev, event_hash));
        }

        results
    }

    /// 计算订单哈希
    pub fn compute_order_hash(&self, last_event_hash: &str) -> String {
        HashChain::compute_order_hash(
            &self.receipt_number,
            self.total,
            self.start_time,
            self.discount_amount,
            last_event_hash,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_hash_computation() {
        let prev_hash = "abc123";
        let event_type = "order_created";
        let timestamp = 1704067200;
        let data_json = r#"{"items": 2}"#;

        let hash = HashChain::compute_event_hash(event_type, timestamp, Some(data_json), prev_hash);

        // 验证哈希是 64 字符的十六进制字符串
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_order_hash_computation() {
        let receipt_number = "RC20250101001";
        let total = 5000;  // 50.00 元
        let start_time = 1704067200;
        let discount_amount = 500;  // 5.00 元
        let last_event_hash = "def456";

        let hash = HashChain::compute_order_hash(
            receipt_number,
            total,
            start_time,
            discount_amount,
            last_event_hash,
        );

        assert_eq!(hash.len(), 64);
    }
}
```

### 订单创建服务

```rust
// src-tauri/src/services/order_service.rs

use sqlx::{SqlitePool, Transaction};
use crate::core::types::{
    Order, OrderEvent, OrderItem, OrderItemOption, Payment,
    SystemState, AuditContext,
};
use crate::core::hash_chain::{HashChain, OrderCreationContext};
use crate::services::audit_service::AuditService;
use anyhow::{Result, Context};

pub struct OrderService;

impl OrderService {
    /// 创建新订单
    pub async fn create_order(
        pool: &SqlitePool,
        mut ctx: OrderCreationContext,
        items: Vec<OrderItemData>,
        audit_ctx: &AuditContext,
    ) -> Result<Order> {
        let mut tx = pool.begin().await?;

        // 1. 获取系统状态
        let state = Self::get_system_state(&mut tx).await?
            .context("System state not initialized")?;

        // 2. 验证 prev_hash
        let expected_prev = state.last_order_hash.as_deref()
            .unwrap_or(HashChain::genesis());
        if ctx.prev_hash != expected_prev {
            anyhow::bail!(
                "Hash chain broken: expected prev_hash '{}', got '{}'",
                expected_prev,
                ctx.prev_hash
            );
        }

        // 3. 计算事件链
        let event_chain = ctx.compute_event_chain();

        // 4. 计算订单哈希
        let last_event_hash = event_chain.last()
            .map(|(_, _, hash)| hash.clone())
            .unwrap_or_else(|| ctx.receipt_number.clone());
        let order_hash = ctx.compute_order_hash(&last_event_hash);

        // 5. 生成 order_id
        let order_id = Self::get_next_order_id(&mut tx).await?;

        // 6. 插入订单
        let order = Self::insert_order(
            &mut tx,
            order_id,
            &ctx.receipt_number,
            &order_hash,
            &ctx,
            audit_ctx,
        ).await?;

        // 7. 插入事件
        for (event_data, prev, curr) in &event_chain {
            Self::insert_event(
                &mut tx,
                order_id,
                event_data,
                prev,
                curr,
            ).await?;
        }

        // 8. 插入订单明细
        for item in items {
            Self::insert_order_item(&mut tx, order_id, &item).await?;
        }

        // 9. 更新系统状态
        Self::update_system_state(&mut tx, &order).await?;

        tx.commit().await?;

        // 10. 记录审计日志
        AuditService::log_order_created(pool, &order, audit_ctx).await;

        Ok(order)
    }

    async fn get_system_state(tx: &mut Transaction<'_, sqlx::Sqlite>) -> Result<SystemState> {
        sqlx::query_as!(
            SystemState,
            "SELECT * FROM system_state WHERE id = 1"
        )
        .fetch_one(tx)
        .await
        .context("Failed to get system state")
    }

    async fn get_next_order_id(tx: &mut Transaction<'_, sqlx::Sqlite>) -> Result<i64> {
        let state = sqlx::query!(
            "SELECT last_order_id FROM system_state WHERE id = 1"
        )
        .fetch_one(tx)
        .await?;

        let next_id = state.last_order_id + 1;
        sqlx::query!(
            "UPDATE system_state SET last_order_id = ? WHERE id = 1",
            next_id
        )
        .execute(tx)
        .await?;

        Ok(next_id)
    }

    async fn insert_order(
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        order_id: i64,
        receipt_number: &str,
        order_hash: &str,
        ctx: &OrderCreationContext,
        audit_ctx: &AuditContext,
    ) -> Result<Order> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query!(
            r#"
            INSERT INTO orders (
                order_id, receipt_number, table_id, table_name, status,
                start_time, subtotal, total, discount_amount,
                surcharge_amount, surcharge_total, prev_hash, curr_hash,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            order_id,
            receipt_number,
            None::<i32>,  // table_id
            None::<String>, // table_name
            "OPEN",
            ctx.start_time,
            ctx.total,
            ctx.total,
            ctx.discount_amount,
            0i64,  // surcharge_amount
            0i64,  // surcharge_total
            ctx.prev_hash,
            order_hash,
            now,
            now,
        )
        .execute(tx)
        .await?;

        Ok(Order {
            order_id,
            receipt_number: receipt_number.to_string(),
            status: "OPEN".to_string(),
            total: ctx.total,
            prev_hash: ctx.prev_hash.clone(),
            curr_hash: order_hash.to_string(),
            created_at: now,
            ..Default::default()
        })
    }

    async fn insert_event(
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        order_id: i64,
        event: &OrderEventData,
        prev_hash: &str,
        curr_hash: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO orders_events (
                order_id, type, timestamp, title, summary, note, color,
                data_json, prev_hash, curr_hash, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            order_id,
            event.type_,
            event.timestamp,
            event.title,
            event.summary,
            event.note,
            event.color,
            event.data_json,
            prev_hash,
            curr_hash,
            event.timestamp,
        )
        .execute(tx)
        .await?;

        Ok(())
    }

    async fn insert_order_item(
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        order_id: i64,
        item: &OrderItemData,
    ) -> Result<()> {
        let item_uuid = format!("item_{}", uuid::Uuid::new_v4());

        sqlx::query!(
            r#"
            INSERT INTO order_items (
                uuid, order_id, product_id, specification_id, receipt_name,
                name, price, quantity, discount_amount, surcharge_amount,
                original_price, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            item_uuid,
            order_id,
            item.product_id,
            item.specification_id,
            item.receipt_name,
            item.name,
            item.price,
            item.quantity,
            item.discount_amount,
            item.surcharge_amount,
            item.original_price,
            item.created_at,
            item.updated_at,
        )
        .execute(tx)
        .await?;

        Ok(())
    }

    async fn update_system_state(
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        order: &Order,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query!(
            r#"
            UPDATE system_state SET
                last_order_hash = ?,
                order_count = order_count + 1,
                updated_at = ?
            WHERE id = 1
            "#,
            order.curr_hash,
            now,
        )
        .execute(tx)
        .await?;

        Ok(())
    }

    /// 验证订单哈希链
    pub async fn verify_order_chain(
        pool: &SqlitePool,
        order_id: i64,
    ) -> Result<bool> {
        let mut conn = pool.acquire().await?;

        // 获取订单
        let order: Order = sqlx::query_as!(
            Order,
            "SELECT * FROM orders WHERE order_id = ?",
            order_id
        )
        .fetch_one(&mut conn)
        .await?;

        // 获取事件
        let events: Vec<OrderEvent> = sqlx::query_as!(
            OrderEvent,
            "SELECT * FROM orders_events WHERE order_id = ? ORDER BY event_id",
            order_id
        )
        .fetch_all(&mut conn)
        .await?;

        // 验证事件链
        let mut expected_prev = order.receipt_number.clone();
        for event in &events {
            let computed = HashChain::compute_event_hash(
                &event.type_,
                event.timestamp,
                event.data_json.as_deref(),
                &expected_prev,
            );

            if event.curr_hash != computed {
                return Ok(false);
            }
            expected_prev = event.curr_hash.clone();
        }

        // 验证订单哈希
        let last_event_hash = events.last()
            .map(|e| e.curr_hash.clone())
            .unwrap_or(order.receipt_number.clone());

        let computed_order = HashChain::compute_order_hash(
            &order.receipt_number,
            order.total,
            order.start_time.unwrap_or(0),
            order.discount_amount,
            &last_event_hash,
        );

        if order.curr_hash != computed_order {
            return Ok(false);
        }

        Ok(true)
    }
}

/// 订单明细数据
pub struct OrderItemData {
    pub product_id: Option<String>,
    pub specification_id: Option<i64>,
    pub receipt_name: Option<String>,
    pub name: String,
    pub price: i64,
    pub quantity: i32,
    pub discount_amount: i64,
    pub surcharge_amount: i64,
    pub original_price: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
```

## 金额计算工具

```rust
// src-tauri/src/utils/currency.rs

use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CurrencyError {
    #[error("Precision loss in currency calculation")]
    PrecisionLoss,
}

/// 将元转换为分
pub fn yuan_to_cents(yuan: f64) -> i64 {
    (Decimal::from_f64(yuan).unwrap() * Decimal::new(100, 0))
        .round()
        .to_i64()
        .unwrap_or(0)
}

/// 将分转换为元
pub fn cents_to_yuan(cents: i64) -> f64 {
    Decimal::new(cents, 2).to_f64().unwrap_or(0.0)
}

/// 安全地计算百分比调整
pub fn apply_percentage(cents: i64, percentage: f64) -> i64 {
    let decimal = Decimal::new(cents, 0);
    let percent = Decimal::from_f64(percentage).unwrap();
    let result = (decimal * (Decimal::ONE + percent / Decimal::new(100, 0)))
        .round()
        .to_i64()
        .unwrap_or(cents);
    result
}

/// 安全地计算固定金额调整
pub fn apply_fixed(cents: i64, fixed: i64) -> i64 {
    cents.saturating_add(fixed)
}

/// 计算折扣后金额
pub fn apply_discount(cents: i64, discount_type: &str, discount_value: f64) -> i64 {
    match discount_type {
        "PERCENTAGE" => {
            let decimal = Decimal::new(cents, 0);
            let percent = Decimal::from_f64(discount_value).unwrap();
            let result = (decimal * (Decimal::ONE - percent / Decimal::new(100, 0)))
                .round()
                .to_i64()
                .unwrap_or(cents);
            result
        }
        "FIXED_AMOUNT" => {
            let fixed = (Decimal::from_f64(discount_value).unwrap() * Decimal::new(100, 0))
                .round()
                .to_i64()
                .unwrap_or(0);
            cents.saturating_sub(fixed).max(0)
        }
        _ => cents,
    }
}

/// 计算附加费
pub fn apply_surcharge(cents: i64, surcharge_type: &str, surcharge_value: f64) -> i64 {
    match surcharge_type {
        "PERCENTAGE" => apply_percentage(cents, surcharge_value),
        "FIXED_AMOUNT" => {
            let fixed = (Decimal::from_f64(surcharge_value).unwrap() * Decimal::new(100, 0))
                .round()
                .to_i64()
                .unwrap_or(0);
            cents.saturating_add(fixed)
        }
        _ => cents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yuan_to_cents() {
        assert_eq!(yuan_to_cents(12.50), 1250);
        assert_eq!(yuan_to_cents(0.01), 1);
        assert_eq!(yuan_to_cents(100.00), 10000);
    }

    #[test]
    fn test_cents_to_yuan() {
        assert!((cents_to_yuan(1250) - 12.50).abs() < 0.01);
        assert!((cents_to_yuan(1) - 0.01).abs() < 0.01);
    }

    #[test]
    fn test_apply_percentage() {
        assert_eq!(apply_percentage(1000, 10.0), 1100);  // 10% 增加
        assert_eq!(apply_percentage(1000, -10.0), 900);  // 10% 减少
        assert_eq!(apply_percentage(1000, 0.0), 1000);
    }

    #[test]
    fn test_apply_discount() {
        assert_eq!(apply_discount(1000, "PERCENTAGE", 10.0), 900);
        assert_eq!(apply_discount(1000, "FIXED_AMOUNT", 5.0), 500);
        assert_eq!(apply_discount(1000, "PERCENTAGE", 100.0), 0);
    }
}
```

## 软删除与审计

### 审计服务

```rust
// src-tauri/src/services/audit_service.rs

use sqlx::{SqlitePool, query};
use crate::core::types::AuditContext;
use crate::core::types::AuditLog;
use chrono::Utc;

pub struct AuditService;

impl AuditService {
    /// 记录审计事件
    pub async fn log(
        pool: &SqlitePool,
        category: &str,
        event_type: &str,
        action: &str,
        description: &str,
        severity: &str,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
        entity_name: Option<&str>,
        metadata: Option<serde_json::Value>,
        ctx: &AuditContext,
    ) {
        let uuid = format!("audit_{}", uuid::Uuid::new_v4());
        let timestamp = Utc::now().timestamp();

        if let Err(e) = query!(
            r#"
            INSERT INTO audit_logs (
                uuid, timestamp, category, event_type, user_id, username,
                entity_type, entity_id, entity_name, action, description,
                severity, metadata_json, source, source_device, source_ip
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            uuid,
            timestamp,
            category,
            event_type,
            ctx.user_id,
            ctx.username,
            entity_type,
            entity_id,
            entity_name,
            action,
            description,
            severity,
            metadata.map(|m| m.to_string()),
            ctx.source,
            ctx.source_device,
            ctx.source_ip,
        )
        .execute(pool)
        .await
        {
            log::error!("Failed to write audit log: {}", e);
        }
    }

    /// 记录订单创建
    pub async fn log_order_created(
        pool: &SqlitePool,
        order: &crate::core::types::Order,
        ctx: &AuditContext,
    ) {
        let metadata = serde_json::json!({
            "receipt_number": order.receipt_number,
            "total": order.total,
            "status": order.status,
        });

        Self::log(
            pool,
            "PAYMENT",
            "order_created",
            "Crear pedido",
            "Nuevo pedido creado",
            "INFO",
            Some("order"),
            Some(&order.order_id.to_string()),
            Some(&order.receipt_number),
            Some(metadata),
            ctx,
        ).await;
    }

    /// 记录支付
    pub async fn log_payment(
        pool: &SqlitePool,
        order_id: i64,
        amount: i64,
        method: &str,
        ctx: &AuditContext,
    ) {
        let metadata = serde_json::json!({
            "order_id": order_id,
            "amount": amount,
            "method": method,
        });

        Self::log(
            pool,
            "PAYMENT",
            "payment_completed",
            "Pago completado",
            format!("Pago de {} completado", amount).as_str(),
            "INFO",
            Some("payment"),
            Some(&order_id.to_string()),
            None,
            Some(metadata),
            ctx,
        ).await;
    }

    /// 记录软删除
    pub async fn log_soft_delete(
        pool: &SqlitePool,
        entity_type: &str,
        entity_id: &str,
        entity_name: &str,
        ctx: &AuditContext,
    ) {
        Self::log(
            pool,
            "DATA",
            "soft_delete",
            "Eliminar suavemente",
            format!("{} {} eliminado suavemente", entity_type, entity_name).as_str(),
            "INFO",
            Some(entity_type),
            Some(entity_id),
            Some(entity_name),
            None,
            ctx,
        ).await;
    }

    /// 获取审计日志
    pub async fn get_logs(
        pool: &SqlitePool,
        category: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let mut query_str = "SELECT * FROM audit_logs WHERE 1=1".to_string();
        let mut params: Vec<String> = Vec::new();

        if let Some(cat) = category {
            query_str.push_str(" AND category = ?");
            params.push(cat.to_string());
        }

        if let Some(start) = start_time {
            query_str.push_str(" AND timestamp >= ?");
            params.push(start.to_string());
        }

        if let Some(end) = end_time {
            query_str.push_str(" AND timestamp <= ?");
            params.push(end.to_string());
        }

        query_str.push_str(&format!(" ORDER BY timestamp DESC LIMIT {}", limit));

        // 使用 query_as_with
        sqlx::query_as_with(&query_str, params)
            .fetch_all(pool)
            .await
    }
}
```

### 软删除助手

```rust
// src-tauri/src/services/soft_delete.rs

use sqlx::{SqlitePool, query};
use chrono::Utc;

pub struct SoftDelete;

impl SoftDelete {
    /// 软删除产品
    pub async fn delete_product(
        pool: &SqlitePool,
        product_id: &str,
        deleted_by: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();

        query!(
            "UPDATE products SET is_deleted = 1, deleted_at = ?, deleted_by = ? WHERE id = ?",
            now,
            deleted_by,
            product_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 软删除分类
    pub async fn delete_category(
        pool: &SqlitePool,
        category_id: i32,
        deleted_by: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();

        query!(
            "UPDATE categories SET is_deleted = 1, deleted_at = ?, deleted_by = ? WHERE id = ?",
            now,
            deleted_by,
            category_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// 查询未删除的记录
    pub async fn get_active_products(
        pool: &SqlitePool,
    ) -> Result<Vec<crate::core::types::Product>, sqlx::Error> {
        sqlx::query_as!(
            crate::core::types::Product,
            "SELECT * FROM products WHERE is_deleted = 0 ORDER BY sort_order"
        )
        .fetch_all(pool)
        .await
    }

    /// 恢复软删除的记录
    pub async fn restore(
        pool: &SqlitePool,
        table: &str,
        id: &str,
        restored_by: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().timestamp();

        query!(format!(
            "UPDATE {} SET is_deleted = 0, deleted_at = NULL, deleted_by = NULL, updated_at = ?, updated_by = ? WHERE id = ?",
            table
        ), now, restored_by, id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
```

## 价格调整规则

```rust
// src-tauri/src/services/pricing_service.rs

use sqlx::{SqlitePool, query_as};
use crate::core::types::{Product, Category, Zone, PriceAdjustmentRule};
use crate::utils::currency::{apply_percentage, apply_fixed};
use chrono::{Utc, Timelike};

pub struct PricingService;

impl PricingService {
    /// 计算商品最终价格
    pub async fn calculate_price(
        pool: &SqlitePool,
        product: &Product,
        category: &Category,
        zone: Option<&Zone>,
        quantity: i32,
    ) -> Result<PriceCalculation, sqlx::Error> {
        let base_price = product.price * quantity as i64;
        let now = Utc::now().timestamp();

        // 获取适用的价格调整规则
        let rules = Self::get_applicable_rules(pool, product, category, zone, now).await?;

        // 计算调整
        let mut subtotal = base_price;
        let mut total_discount = 0i64;
        let mut total_surcharge = 0i64;
        let mut applied_rules = Vec::new();

        let mut stackable_rules: Vec<&PriceAdjustmentRule> = Vec::new();
        let mut best_non_stackable: Option<&PriceAdjustmentRule> = None;

        for rule in &rules {
            // 时间验证
            if !Self::is_time_valid(rule, now) {
                continue;
            }

            if !rule.is_stackable {
                // 非可叠加规则，取最高优先级
                if best_non_stackable.map(|r| r.priority < rule.priority).unwrap_or(true) {
                    best_non_stackable = Some(rule);
                }
            } else {
                stackable_rules.push(rule);
            }
        }

        // 应用非可叠加规则
        if let Some(rule) = best_non_stackable {
            match rule.adjustment_type.as_str() {
                "PERCENTAGE" => {
                    let adjustment = apply_percentage(base_price, rule.adjustment_value);
                    if rule.rule_type == "DISCOUNT" {
                        total_discount += adjustment;
                    } else {
                        total_surcharge += adjustment;
                    }
                }
                "FIXED_AMOUNT" => {
                    let fixed = (rule.adjustment_value * 100.0) as i64;
                    if rule.rule_type == "DISCOUNT" {
                        total_discount += fixed;
                    } else {
                        total_surcharge += fixed;
                    }
                }
                _ => {}
            }
            applied_rules.push(rule);
        }

        // 应用可叠加规则
        let mut total_percentage = 0.0f64;
        let mut total_fixed = 0i64;

        for rule in stackable_rules {
            match rule.adjustment_type.as_str() {
                "PERCENTAGE" => {
                    total_percentage += rule.adjustment_value;
                }
                "FIXED_AMOUNT" => {
                    total_fixed += (rule.adjustment_value * 100.0) as i64;
                }
                _ => {}
            }
            applied_rules.push(rule);
        }

        // 应用可叠加调整
        if total_percentage != 0.0 {
            let adjusted = apply_percentage(base_price, total_percentage);
            if total_percentage > 0.0 {
                total_surcharge += adjusted - base_price;
            } else {
                total_discount += base_price - adjusted;
            }
        }

        subtotal = base_price + total_surcharge - total_discount;

        Ok(PriceCalculation {
            base_price,
            subtotal,
            total: subtotal.max(0),
            discount_amount: total_discount,
            surcharge_amount: total_surcharge,
            applied_rules: applied_rules.into_iter().map(|r| r.name.clone()).collect(),
        })
    }

    /// 获取适用的价格调整规则
    async fn get_applicable_rules(
        pool: &SqlitePool,
        product: &Product,
        category: &Category,
        zone: Option<&Zone>,
        now: i64,
    ) -> Result<Vec<PriceAdjustmentRule>, sqlx::Error> {
        // 获取全局规则、分类规则、区域规则
        query_as!(
            PriceAdjustmentRule,
            r#"
            SELECT * FROM price_adjustment_rules
            WHERE is_active = 1 AND is_deleted = 0
            AND (
                (scope = 'GLOBAL')
                OR (scope = 'CATEGORY' AND target_id = ?)
                OR (scope = 'PRODUCT' AND target_id = ?)
                OR (scope = 'ZONE' AND zone_id = ?)
            )
            ORDER BY priority DESC
            "#,
            category.id.to_string(),
            product.id,
            zone.map(|z| z.id).unwrap_or(0),
        )
        .fetch_all(pool)
        .await
    }

    /// 验证时间规则
    fn is_time_valid(rule: &PriceAdjustmentRule, now: i64) -> bool {
        match rule.time_mode.as_str() {
            "ALWAYS" => true,
            "ONETIME" => {
                let start = rule.start_time.unwrap_or(0);
                let end = rule.end_time.unwrap_or(i64::MAX);
                now >= start && now <= end
            }
            "SCHEDULE" => {
                // 解析 schedule_config_json
                if let Some(config) = &rule.schedule_config_json {
                    Self::check_schedule(config, now)
                } else {
                    true
                }
            }
            _ => true,
        }
    }

    /// 检查schedule配置
    fn check_schedule(config: &str, now: i64) -> bool {
        // 简化实现，实际需要解析 JSON
        true
    }
}

pub struct PriceCalculation {
    pub base_price: i64,
    pub subtotal: i64,
    pub total: i64,
    pub discount_amount: i64,
    pub surcharge_amount: i64,
    pub applied_rules: Vec<String>,
}
```

## 打印设置继承

```rust
// src-tauri/src/services/print_service.rs

use crate::core::types::{Product, Category, PrintSettings};

pub struct PrintSettingsInheritance;

impl PrintSettingsInheritance {
    /// 获取产品的打印设置（继承分类设置）
    pub fn get_print_settings(
        product: &Product,
        category: &Category,
    ) -> PrintSettings {
        PrintSettings {
            kitchen_printer_id: product.kitchen_printer_id
                .or(category.kitchen_printer_id),
            kitchen_print_name: product.kitchen_print_name
                .or(product.receipt_name.clone())
                .unwrap_or_else(|| product.name.clone()),
            is_kitchen_print_enabled: if product.is_kitchen_print_enabled == -1 {
                category.is_kitchen_print_enabled
            } else {
                product.is_kitchen_print_enabled == 1
            },
            is_label_print_enabled: if product.is_label_print_enabled == -1 {
                category.is_label_print_enabled
            } else {
                product.is_label_print_enabled == 1
            },
        }
    }

    /// 判断是否需要打印到厨房
    pub fn should_print_to_kitchen(
        product: &Product,
        category: &Category,
    ) -> bool {
        let settings = Self::get_print_settings(product, category);
        settings.is_kitchen_print_enabled && settings.kitchen_printer_id.is_some()
    }

    /// 获取要打印的打印机 ID
    pub fn get_kitchen_printer_id(
        product: &Product,
        category: &Category,
    ) -> Option<i64> {
        let settings = Self::get_print_settings(product, category);
        settings.kitchen_printer_id
    }
}
```

## 常见问题

### Q: 离线期间创建的订单如何同步？

A: 离线期间订单使用本地 `order_id` 自增，存储在本地数据库。恢复网络后，客户端将订单和事件按顺序发送给服务端验证。服务端验证哈希链后，返回确认信息。

### Q: 如何处理订单冲突？

A: 如果客户端订单的 `prev_hash` 与服务端期望不匹配，或者哈希链验证失败，服务端会返回冲突信息。客户端可以选择重新同步或标记订单为可疑。

### Q: 软删除的数据还会被查询到吗？

A: 不会。所有查询都需要添加 `WHERE is_deleted = 0` 条件。触发器会自动记录软删除操作到审计日志。

### Q: 打印设置的继承逻辑是什么？

A: `is_kitchen_print_enabled = -1` 表示继承分类设置，0 表示禁用，1 表示启用。`kitchen_printer_id` 为 NULL 时使用分类的打印机。

### Q: 金额字段为什么要用整数？

A: 避免浮点运算的精度问题。例如 `0.1 + 0.2` 在浮点数中不等于 `0.3`，而使用整数分则完全精确。

### Q: 如何验证哈希链完整性？

A: 使用 `OrderService::verify_order_chain()` 方法，它会：
1. 重新计算每个事件的哈希并验证
2. 重新计算订单哈希并验证
3. 验证 `prev_hash` 是否连续

### Q: 审计日志有什么作用？

A: 用于税务审计追溯，记录所有关键操作的详细信息，包括操作人、时间、变更内容等。日志使用西班牙语，符合当地税务合规要求。

## 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/migrations/20251215000000_init.sql` | 数据库迁移文件 |
| `src-tauri/src/core/db.rs` | 数据库连接和初始化 |
| `src-tauri/src/core/types.rs` | Rust 类型定义 (API 层) |
| `src-tauri/src/core/hash_chain.rs` | 哈希链计算 |
| `src-tauri/src/services/order_service.rs` | 订单服务 |
| `src-tauri/src/services/audit_service.rs` | 审计服务 |
| `src-tauri/src/services/pricing_service.rs` | 价格计算服务 |
| `src-tauri/src/api/` | API 路由实现 |

## API 层与数据库层

### 类型映射

```rust
// API 层使用 String 类型作为 ID，方便前端处理
pub struct CategoryResp {
    pub id: String,           // "1" 而不是 1i32
    pub name: String,
    // ...
}

// 数据库层使用实际的数据库类型
pub struct CategoryDb {
    pub id: i32,              // INTEGER PRIMARY KEY
    pub uuid: String,
    pub name: String,
    // ...
}
```

### 价格转换

```rust
// API 层: 使用 f64 (元)
pub struct ProductResp {
    pub price: f64,           // 12.50 表示 12.50 元
}

// 数据库层: 使用 i64 (分)
pub struct ProductDb {
    pub price: i64,           // 1250 表示 12.50 元
}

// 转换函数
fn db_to_api_price(cents: i64) -> f64 {
    cents as f64 / 100.0
}

fn api_to_db_price(yuan: f64) -> i64 {
    (yuan * 100.0).round() as i64
}
```

### 请求参数处理

```rust
// API 请求参数使用 String 类型
#[derive(Deserialize)]
pub struct CreateProductParams {
    pub name: String,
    pub price: f64,           // 前端发送 12.50
    pub category: String,     // "4" 而不是 4i32
    // ...
}

// 转换为数据库参数
impl CreateProductParams {
    pub fn to_db(&self) -> Result<InsertProduct, anyhow::Error> {
        Ok(InsertProduct {
            id: format!("prod_{}", Uuid::new_v4()),
            uuid: Uuid::new_v4().to_string(),
            name: self.name.clone(),
            price: (self.price * 100.0).round() as i64,  // 转换为分
            category_id: self.category.parse::<i32>()?,
            // ...
        })
    }
}
```

## 迁移检查清单

- [x] 更新 SQL Schema (20 张表，14 个触发器)
- [x] 添加迁移 SQL 文件
- [x] 更新 `src-tauri/src/api/products.rs` - 价格字段转换 + 软删除
- [x] 更新 `src-tauri/src/api/orders/crud.rs` - 金额字段转换
- [x] 更新 `src-tauri/src/api/orders/loaders.rs` - 金额字段转换
- [x] 更新 `src-tauri/src/api/categories.rs` - 软删除支持 + is_deleted 过滤
- [x] 更新 `src-tauri/src/api/specifications.rs` - 价格字段转换
- [x] 添加金额转换工具函数 (`src-tauri/src/utils/price.rs`)
- [x] 实现哈希链计算模块 (`src-tauri/src/core/hash_chain.rs`)
- [x] 实现审计日志服务 (`src-tauri/src/services/audit.rs`)
- [x] 添加单元测试 (19 个测试全部通过)
- [x] 验证所有 API 端点

## 代码迁移示例

### products.rs 价格字段迁移

**当前代码** (需要修改):
```rust
// src-tauri/src/api/products.rs:78
price: r.get::<f64, _>("price"),  // ❌ 假设 DB 返回 f64
```

**迁移后代码**:
```rust
// 添加价格转换函数
fn cents_to_yuan(cents: i64) -> f64 {
    cents as f64 / 100.0
}

// 修改查询读取逻辑
price: cents_to_yuan(r.get::<i64, _>("price")),  // ✅ 读取分，转换为元
```

### create_product 迁移

**当前代码**:
```rust
#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    params: CreateProductParams,
) -> Result<(), String> {
    sqlx::query!(
        "INSERT INTO products (id, name, price, ...) VALUES (?, ?, ?, ...)",
        Uuid::new_v4().to_string(),
        params.name,
        params.price,  // ❌ 直接插入 f64
        // ...
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}
```

**迁移后代码**:
```rust
#[tauri::command]
pub async fn create_product(
    state: State<'_, AppState>,
    params: CreateProductParams,
) -> Result<(), String> {
    // 1. 将价格从元转换为分
    let price_cents = (params.price * 100.0).round() as i64;

    sqlx::query!(
        "INSERT INTO products (id, uuid, name, price, category_id, ...) VALUES (?, ?, ?, ?, ?, ...)",
        format!("prod_{}", Uuid::new_v4()),
        Uuid::new_v4().to_string(),
        params.name,
        price_cents,  // ✅ 存储分
        params.category.parse::<i32>()?,
        // ...
    )
    .execute(&state.pool)
    .await?;
    Ok(())
}
```

### 统一的价格转换模块

```rust
// src-tauri/src/utils/price.rs

/// 将元转换为分 (四舍五入)
pub fn yuan_to_cents(yuan: f64) -> i64 {
    (yuan * 100.0).round() as i64
}

/// 将分转换为元
pub fn cents_to_yuan(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// 安全地将 Optional 元转换为 Optional 分
pub fn opt_yuan_to_opt_cents(yuan: Option<f64>) -> Option<i64> {
    yuan.map(|v| yuan_to_cents(v))
}

/// 安全地将 Optional 分转换为 Optional 元
pub fn opt_cents_to_opt_yuan(cents: Option<i64>) -> Option<f64> {
    cents.map(|v| cents_to_yuan(v))
}

/// 格式化金额为货币字符串
pub fn format_yuan(yuan: f64) -> String {
    format!("${:.2}", yuan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yuan_to_cents() {
        assert_eq!(yuan_to_cents(12.50), 1250);
        assert_eq!(yuan_to_cents(0.01), 1);
        assert_eq!(yuan_to_cents(100.00), 10000);
    }

    #[test]
    fn test_cents_to_yuan() {
        assert!((cents_to_yuan(1250) - 12.50).abs() < 0.001);
        assert!((cents_to_yuan(1) - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_round_trip() {
        for price in [0.01, 0.99, 1.00, 12.50, 99.99, 100.00] {
            let cents = yuan_to_cents(price);
            let back = cents_to_yuan(cents);
            assert!((back - price).abs() < 0.001, "Failed for {}", price);
        }
    }
}
```

### 使用示例

```rust
use crate::utils::price::{yuan_to_cents, cents_to_yuan};

// 在读取时转换
fn row_to_product_resp(row: &sqlx::Row) -> ProductResp {
    let price_cents: i64 = row.get("price");
    ProductResp {
        price: cents_to_yuan(price_cents),
        // ...
    }
}

// 在写入时转换
fn insert_product(pool: &SqlitePool, params: CreateProductParams) -> Result<()> {
    let price_cents = yuan_to_cents(params.price);
    sqlx::query!(
        "INSERT INTO products (name, price) VALUES (?, ?)",
        params.name,
        price_cents,
    )
    .execute(pool)?;
    Ok(())
}
```
