# 厨房打印功能实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现下单自动打印厨房单和标签功能，支持按打印目的地分组发送到各厨房/出菜口

**Architecture:**
- `edge-server` 新增 `printing` 模块，监听 `ItemsAdded` 事件
- 使用 redb 存储 `KitchenOrder` 和 `LabelPrintRecord`
- 内存缓存商品/分类打印配置，支持回退链路由

**Tech Stack:** Rust (edge-server), redb, Axum HTTP API, TypeScript (red_coral)

---

## Phase 1: 数据模型扩展

### Task 1: EmbeddedPrinter 添加 printer_format 字段

**Files:**
- Modify: `shared/src/models/print_destination.rs:5-13`
- Modify: `red_coral/src/core/domain/types/api/models.ts:266-273`

**Step 1: 修改 Rust EmbeddedPrinter 结构体**

在 `shared/src/models/print_destination.rs` 中添加 `printer_format` 字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPrinter {
    pub printer_type: String, // "network" | "driver"
    /// Printer format: "escpos" (厨房单/小票) | "label" (标签)
    #[serde(default = "default_printer_format")]
    pub printer_format: String,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub driver_name: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}

fn default_printer_format() -> String {
    "escpos".to_string()
}
```

**Step 2: 验证 Rust 编译**

Run: `cargo check -p shared`
Expected: 编译成功

**Step 3: 更新 TypeScript 类型**

在 `red_coral/src/core/domain/types/api/models.ts` 中更新：

```typescript
export type PrinterType = 'network' | 'driver';
export type PrinterFormat = 'escpos' | 'label';

export interface EmbeddedPrinter {
  printer_type: PrinterType;
  /** Printer format: escpos (厨房单/小票) | label (标签) */
  printer_format: PrinterFormat;
  ip?: string;
  port?: number;
  driver_name?: string;
  priority: number;
  is_active: boolean;
}
```

**Step 4: 验证 TypeScript 编译**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 编译成功

**Step 5: Commit**

```bash
git add shared/src/models/print_destination.rs red_coral/src/core/domain/types/api/models.ts
git commit -m "$(cat <<'EOF'
feat(printing): add printer_format field to EmbeddedPrinter

Support escpos (kitchen/receipt) and label printer formats.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Category 模型添加 label_print_destinations 字段

**Files:**
- Modify: `shared/src/models/category.rs`
- Modify: `red_coral/src/core/domain/types/api/models.ts`

**Step 1: 修改 Rust Category 结构体**

在 `shared/src/models/category.rs` 中，将 `print_destinations` 重命名为 `kitchen_print_destinations`，并添加 `label_print_destinations`：

```rust
/// Category entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<String>,
    pub name: String,
    pub sort_order: i32,
    /// Kitchen print destination references (String IDs)
    #[serde(default)]
    pub kitchen_print_destinations: Vec<String>,
    /// Label print destination references (String IDs)
    #[serde(default)]
    pub label_print_destinations: Vec<String>,
    /// Whether kitchen printing is enabled for this category
    #[serde(default)]
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
    pub is_active: bool,
    /// Whether this is a virtual category
    #[serde(default)]
    pub is_virtual: bool,
    /// Tag IDs for virtual category filtering
    #[serde(default)]
    pub tag_ids: Vec<String>,
    /// Match mode for virtual category: "any" or "all"
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
}
```

同步更新 `CategoryCreate` 和 `CategoryUpdate` 结构体。

**Step 2: 验证 Rust 编译**

Run: `cargo check -p shared`
Expected: 编译成功

**Step 3: 更新 TypeScript 类型**

```typescript
export interface Category {
  id: string | null;
  name: string;
  sort_order: number;
  /** Kitchen print destinations */
  kitchen_print_destinations: string[];
  /** Label print destinations */
  label_print_destinations: string[];
  is_kitchen_print_enabled: boolean;
  is_label_print_enabled: boolean;
  is_active: boolean;
  is_virtual: boolean;
  tag_ids: string[];
  match_mode: 'any' | 'all';
}
```

同步更新 `CategoryCreate` 和 `CategoryUpdate`。

**Step 4: 验证 TypeScript 编译**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 编译成功

**Step 5: Commit**

```bash
git add shared/src/models/category.rs red_coral/src/core/domain/types/api/models.ts
git commit -m "$(cat <<'EOF'
feat(category): separate kitchen and label print destinations

Rename print_destinations to kitchen_print_destinations, add label_print_destinations.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Product 模型添加 label_print_destinations 字段

**Files:**
- Modify: `shared/src/models/product.rs`
- Modify: `red_coral/src/core/domain/types/api/models.ts`

**Step 1: 修改 Rust Product 结构体**

在 `shared/src/models/product.rs` 中，将 `print_destinations` 重命名为 `kitchen_print_destinations`，并添加 `label_print_destinations`：

```rust
/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<String>,
    pub name: String,
    pub image: String,
    pub category: String,
    pub sort_order: i32,
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// Kitchen print destinations
    #[serde(default)]
    pub kitchen_print_destinations: Vec<String>,
    /// Label print destinations
    #[serde(default)]
    pub label_print_destinations: Vec<String>,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub specs: Vec<EmbeddedSpec>,
}
```

同步更新 `ProductCreate`、`ProductUpdate`、`ProductFull`。

**Step 2: 验证 Rust 编译**

Run: `cargo check -p shared`
Expected: 编译成功

**Step 3: 更新 TypeScript 类型**

```typescript
export interface Product {
  id: string | null;
  name: string;
  image: string;
  category: string;
  sort_order: number;
  tax_rate: number;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  /** Kitchen print destinations */
  kitchen_print_destinations: string[];
  /** Label print destinations */
  label_print_destinations: string[];
  is_label_print_enabled: PrintState;
  is_active: boolean;
  tags: string[];
  specs: EmbeddedSpec[];
}
```

同步更新相关接口。

**Step 4: 验证 TypeScript 编译**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 编译成功

**Step 5: Commit**

```bash
git add shared/src/models/product.rs red_coral/src/core/domain/types/api/models.ts
git commit -m "$(cat <<'EOF'
feat(product): separate kitchen and label print destinations

Rename print_destinations to kitchen_print_destinations, add label_print_destinations.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2: Printing 模块基础设施

### Task 4: 创建 printing 模块类型定义

**Files:**
- Create: `edge-server/src/printing/mod.rs`
- Create: `edge-server/src/printing/types.rs`
- Modify: `edge-server/src/lib.rs`

**Step 1: 创建 types.rs**

创建 `edge-server/src/printing/types.rs`：

```rust
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
    pub is_label_print_enabled: bool,
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
```

**Step 2: 创建 mod.rs**

创建 `edge-server/src/printing/mod.rs`：

```rust
//! Kitchen and Label Printing Module
//!
//! This module handles automatic printing on ItemsAdded events:
//! - Kitchen printing: grouped by destination, sent to kitchen printers
//! - Label printing: per-item labels (e.g., bubble tea stickers)

pub mod types;

pub use types::*;
```

**Step 3: 在 lib.rs 注册模块**

在 `edge-server/src/lib.rs` 添加：

```rust
pub mod printing;
```

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 5: Commit**

```bash
git add edge-server/src/printing/ edge-server/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(printing): add types for kitchen/label printing module

Define KitchenOrder, LabelPrintRecord, and print config cache types.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: 实现 PrintConfigCache

**Files:**
- Create: `edge-server/src/printing/cache.rs`
- Modify: `edge-server/src/printing/mod.rs`

**Step 1: 创建 cache.rs**

创建 `edge-server/src/printing/cache.rs`：

```rust
//! Print configuration cache with fallback routing

use super::types::{CategoryPrintConfig, ProductPrintConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 打印配置缓存
#[derive(Debug, Clone)]
pub struct PrintConfigCache {
    inner: Arc<RwLock<PrintConfigCacheInner>>,
}

#[derive(Debug, Default)]
struct PrintConfigCacheInner {
    products: HashMap<String, ProductPrintConfig>,
    categories: HashMap<String, CategoryPrintConfig>,
    /// 系统默认厨房打印机（最终回退）
    default_kitchen_printer: Option<String>,
    /// 系统默认标签打印机（最终回退）
    default_label_printer: Option<String>,
}

impl PrintConfigCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PrintConfigCacheInner::default())),
        }
    }

    /// 厨房打印功能是否启用
    pub async fn is_kitchen_print_enabled(&self) -> bool {
        let inner = self.inner.read().await;
        inner.default_kitchen_printer.is_some()
    }

    /// 标签打印功能是否启用
    pub async fn is_label_print_enabled(&self) -> bool {
        let inner = self.inner.read().await;
        inner.default_label_printer.is_some()
    }

    /// 设置系统默认打印机
    pub async fn set_defaults(
        &self,
        kitchen: Option<String>,
        label: Option<String>,
    ) {
        let mut inner = self.inner.write().await;
        inner.default_kitchen_printer = kitchen;
        inner.default_label_printer = label;
    }

    /// 更新商品配置
    pub async fn update_product(&self, config: ProductPrintConfig) {
        let mut inner = self.inner.write().await;
        inner.products.insert(config.product_id.clone(), config);
    }

    /// 更新分类配置
    pub async fn update_category(&self, config: CategoryPrintConfig) {
        let mut inner = self.inner.write().await;
        inner.categories.insert(config.category_id.clone(), config);
    }

    /// 移除商品配置
    pub async fn remove_product(&self, product_id: &str) {
        let mut inner = self.inner.write().await;
        inner.products.remove(product_id);
    }

    /// 移除分类配置
    pub async fn remove_category(&self, category_id: &str) {
        let mut inner = self.inner.write().await;
        inner.categories.remove(category_id);
    }

    /// 获取商品配置
    pub async fn get_product(&self, product_id: &str) -> Option<ProductPrintConfig> {
        let inner = self.inner.read().await;
        inner.products.get(product_id).cloned()
    }

    /// 获取分类配置
    pub async fn get_category(&self, category_id: &str) -> Option<CategoryPrintConfig> {
        let inner = self.inner.read().await;
        inner.categories.get(category_id).cloned()
    }

    /// 获取厨房打印目的地（商品 > 分类 > 系统默认）
    pub async fn get_kitchen_destinations(&self, product_id: &str) -> Vec<String> {
        let inner = self.inner.read().await;

        if let Some(product) = inner.products.get(product_id) {
            // 商品有配置
            if !product.kitchen_print_destinations.is_empty() {
                return product.kitchen_print_destinations.clone();
            }
            // 回退到分类
            if let Some(category) = inner.categories.get(&product.category_id) {
                if category.is_kitchen_print_enabled
                    && !category.kitchen_print_destinations.is_empty()
                {
                    return category.kitchen_print_destinations.clone();
                }
            }
        }

        // 最终回退到系统默认
        inner
            .default_kitchen_printer
            .iter()
            .cloned()
            .collect()
    }

    /// 获取标签打印目的地（商品 > 分类 > 系统默认）
    pub async fn get_label_destinations(&self, product_id: &str) -> Vec<String> {
        let inner = self.inner.read().await;

        if let Some(product) = inner.products.get(product_id) {
            // 检查是否启用标签打印
            let category_enabled = inner
                .categories
                .get(&product.category_id)
                .map(|c| c.is_label_print_enabled)
                .unwrap_or(false);

            let enabled = product.is_label_print_enabled || category_enabled;
            if !enabled {
                return vec![];
            }

            // 商品有配置
            if !product.label_print_destinations.is_empty() {
                return product.label_print_destinations.clone();
            }
            // 回退到分类
            if let Some(category) = inner.categories.get(&product.category_id) {
                if !category.label_print_destinations.is_empty() {
                    return category.label_print_destinations.clone();
                }
            }
        }

        // 最终回退到系统默认
        inner.default_label_printer.iter().cloned().collect()
    }

    /// 清空缓存
    pub async fn clear(&self) {
        let mut inner = self.inner.write().await;
        inner.products.clear();
        inner.categories.clear();
    }
}

impl Default for PrintConfigCache {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: 更新 mod.rs**

```rust
pub mod cache;
pub mod types;

pub use cache::PrintConfigCache;
pub use types::*;
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 4: Commit**

```bash
git add edge-server/src/printing/
git commit -m "$(cat <<'EOF'
feat(printing): implement PrintConfigCache with fallback routing

Supports product > category > system default fallback for destinations.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: 实现 KitchenOrder redb 存储

**Files:**
- Create: `edge-server/src/printing/storage.rs`
- Modify: `edge-server/src/printing/mod.rs`

**Step 1: 创建 storage.rs**

创建 `edge-server/src/printing/storage.rs`：

```rust
//! redb-based storage for kitchen orders and label records

use super::types::{KitchenOrder, LabelPrintRecord};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition, WriteTransaction};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Kitchen orders table: key = kitchen_order_id, value = JSON
const KITCHEN_ORDERS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("kitchen_orders");

/// Index: (order_id, kitchen_order_id) -> ()
const KITCHEN_ORDERS_BY_ORDER_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("kitchen_orders_by_order");

/// Label records table: key = label_record_id, value = JSON
const LABEL_RECORDS_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("label_records");

/// Index: (order_id, label_record_id) -> ()
const LABEL_RECORDS_BY_ORDER_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("label_records_by_order");

#[derive(Debug, Error)]
pub enum PrintStorageError {
    #[error("Database error: {0}")]
    Database(#[from] redb::DatabaseError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("Table error: {0}")]
    Table(#[from] redb::TableError),

    #[error("Storage error: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("Commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Kitchen order not found: {0}")]
    KitchenOrderNotFound(String),

    #[error("Label record not found: {0}")]
    LabelRecordNotFound(String),
}

pub type PrintStorageResult<T> = Result<T, PrintStorageError>;

/// Kitchen/Label printing storage
#[derive(Clone)]
pub struct PrintStorage {
    db: Arc<Database>,
}

impl PrintStorage {
    /// Open or create database
    pub fn open(path: impl AsRef<Path>) -> PrintStorageResult<Self> {
        let db = Database::create(path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(KITCHEN_ORDERS_TABLE)?;
            let _ = write_txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Open in-memory database (for testing)
    #[cfg(test)]
    pub fn open_in_memory() -> PrintStorageResult<Self> {
        let db = Database::builder()
            .create_with_backend(redb::backends::InMemoryBackend::new())?;

        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(KITCHEN_ORDERS_TABLE)?;
            let _ = write_txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    pub fn begin_write(&self) -> PrintStorageResult<WriteTransaction> {
        Ok(self.db.begin_write()?)
    }

    // ========== Kitchen Orders ==========

    /// Store a kitchen order
    pub fn store_kitchen_order(
        &self,
        txn: &WriteTransaction,
        order: &KitchenOrder,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;
        let value = serde_json::to_vec(order)?;
        table.insert(order.id.as_str(), value.as_slice())?;

        // Update index
        let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        idx_table.insert((order.order_id.as_str(), order.id.as_str()), ())?;

        Ok(())
    }

    /// Get a kitchen order by ID
    pub fn get_kitchen_order(&self, id: &str) -> PrintStorageResult<Option<KitchenOrder>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;

        match table.get(id)? {
            Some(value) => {
                let order: KitchenOrder = serde_json::from_slice(value.value())?;
                Ok(Some(order))
            }
            None => Ok(None),
        }
    }

    /// Get kitchen orders for an order
    pub fn get_kitchen_orders_for_order(
        &self,
        order_id: &str,
    ) -> PrintStorageResult<Vec<KitchenOrder>> {
        let read_txn = self.db.begin_read()?;
        let idx_table = read_txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        let data_table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;

        let mut orders = Vec::new();
        let range_start = (order_id, "");
        let range_end = (order_id, "\u{ffff}");

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let kitchen_order_id = key.value().1;
            if let Some(value) = data_table.get(kitchen_order_id)? {
                let order: KitchenOrder = serde_json::from_slice(value.value())?;
                orders.push(order);
            }
        }

        orders.sort_by_key(|o| o.created_at);
        Ok(orders)
    }

    /// Get all kitchen orders (paginated)
    pub fn get_all_kitchen_orders(
        &self,
        offset: usize,
        limit: usize,
    ) -> PrintStorageResult<Vec<KitchenOrder>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;

        let mut orders: Vec<KitchenOrder> = Vec::new();
        for result in table.iter()? {
            let (_, value) = result?;
            let order: KitchenOrder = serde_json::from_slice(value.value())?;
            orders.push(order);
        }

        // Sort by created_at descending
        orders.sort_by_key(|o| std::cmp::Reverse(o.created_at));

        Ok(orders.into_iter().skip(offset).take(limit).collect())
    }

    /// Update kitchen order print count
    pub fn increment_kitchen_order_print_count(
        &self,
        txn: &WriteTransaction,
        id: &str,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;

        let value = table
            .get(id)?
            .ok_or_else(|| PrintStorageError::KitchenOrderNotFound(id.to_string()))?;

        let mut order: KitchenOrder = serde_json::from_slice(value.value())?;
        order.print_count += 1;

        let new_value = serde_json::to_vec(&order)?;
        table.insert(id, new_value.as_slice())?;

        Ok(())
    }

    /// Delete kitchen orders for an order
    pub fn delete_kitchen_orders_for_order(
        &self,
        txn: &WriteTransaction,
        order_id: &str,
    ) -> PrintStorageResult<()> {
        let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        let mut data_table = txn.open_table(KITCHEN_ORDERS_TABLE)?;

        // Collect IDs to delete
        let range_start = (order_id, "");
        let range_end = (order_id, "\u{ffff}");
        let mut ids_to_delete = Vec::new();

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            ids_to_delete.push(key.value().1.to_string());
        }

        // Delete from both tables
        for id in &ids_to_delete {
            data_table.remove(id.as_str())?;
            idx_table.remove((order_id, id.as_str()))?;
        }

        Ok(())
    }

    // ========== Label Records ==========

    /// Store a label record
    pub fn store_label_record(
        &self,
        txn: &WriteTransaction,
        record: &LabelPrintRecord,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;
        let value = serde_json::to_vec(record)?;
        table.insert(record.id.as_str(), value.as_slice())?;

        let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        idx_table.insert((record.order_id.as_str(), record.id.as_str()), ())?;

        Ok(())
    }

    /// Get a label record by ID
    pub fn get_label_record(&self, id: &str) -> PrintStorageResult<Option<LabelPrintRecord>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(LABEL_RECORDS_TABLE)?;

        match table.get(id)? {
            Some(value) => {
                let record: LabelPrintRecord = serde_json::from_slice(value.value())?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Get label records for an order
    pub fn get_label_records_for_order(
        &self,
        order_id: &str,
    ) -> PrintStorageResult<Vec<LabelPrintRecord>> {
        let read_txn = self.db.begin_read()?;
        let idx_table = read_txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        let data_table = read_txn.open_table(LABEL_RECORDS_TABLE)?;

        let mut records = Vec::new();
        let range_start = (order_id, "");
        let range_end = (order_id, "\u{ffff}");

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let record_id = key.value().1;
            if let Some(value) = data_table.get(record_id)? {
                let record: LabelPrintRecord = serde_json::from_slice(value.value())?;
                records.push(record);
            }
        }

        records.sort_by_key(|r| r.created_at);
        Ok(records)
    }

    /// Increment label record print count
    pub fn increment_label_record_print_count(
        &self,
        txn: &WriteTransaction,
        id: &str,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;

        let value = table
            .get(id)?
            .ok_or_else(|| PrintStorageError::LabelRecordNotFound(id.to_string()))?;

        let mut record: LabelPrintRecord = serde_json::from_slice(value.value())?;
        record.print_count += 1;

        let new_value = serde_json::to_vec(&record)?;
        table.insert(id, new_value.as_slice())?;

        Ok(())
    }

    /// Delete label records for an order
    pub fn delete_label_records_for_order(
        &self,
        txn: &WriteTransaction,
        order_id: &str,
    ) -> PrintStorageResult<()> {
        let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        let mut data_table = txn.open_table(LABEL_RECORDS_TABLE)?;

        let range_start = (order_id, "");
        let range_end = (order_id, "\u{ffff}");
        let mut ids_to_delete = Vec::new();

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            ids_to_delete.push(key.value().1.to_string());
        }

        for id in &ids_to_delete {
            data_table.remove(id.as_str())?;
            idx_table.remove((order_id, id.as_str()))?;
        }

        Ok(())
    }

    // ========== Cleanup ==========

    /// Clean up old records (older than max_age_secs)
    pub fn cleanup_old_records(&self, max_age_secs: i64) -> PrintStorageResult<usize> {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - max_age_secs;

        let txn = self.db.begin_write()?;
        let mut deleted = 0;

        // Kitchen orders
        {
            let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;
            let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;

            let mut to_delete = Vec::new();
            for result in table.iter()? {
                let (key, value) = result?;
                let order: KitchenOrder = serde_json::from_slice(value.value())?;
                if order.created_at < cutoff {
                    to_delete.push((key.value().to_string(), order.order_id.clone()));
                }
            }

            for (id, order_id) in &to_delete {
                table.remove(id.as_str())?;
                idx_table.remove((order_id.as_str(), id.as_str()))?;
                deleted += 1;
            }
        }

        // Label records
        {
            let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;
            let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;

            let mut to_delete = Vec::new();
            for result in table.iter()? {
                let (key, value) = result?;
                let record: LabelPrintRecord = serde_json::from_slice(value.value())?;
                if record.created_at < cutoff {
                    to_delete.push((key.value().to_string(), record.order_id.clone()));
                }
            }

            for (id, order_id) in &to_delete {
                table.remove(id.as_str())?;
                idx_table.remove((order_id.as_str(), id.as_str()))?;
                deleted += 1;
            }
        }

        txn.commit()?;
        Ok(deleted)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> PrintStorageResult<PrintStorageStats> {
        let read_txn = self.db.begin_read()?;
        let ko_table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;
        let lr_table = read_txn.open_table(LABEL_RECORDS_TABLE)?;

        Ok(PrintStorageStats {
            kitchen_order_count: ko_table.len()?,
            label_record_count: lr_table.len()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PrintStorageStats {
    pub kitchen_order_count: u64,
    pub label_record_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kitchen_order_crud() {
        let storage = PrintStorage::open_in_memory().unwrap();

        let order = KitchenOrder {
            id: "ko-1".to_string(),
            order_id: "order-1".to_string(),
            table_name: Some("Table 1".to_string()),
            created_at: chrono::Utc::now().timestamp(),
            items: vec![],
            print_count: 0,
        };

        let txn = storage.begin_write().unwrap();
        storage.store_kitchen_order(&txn, &order).unwrap();
        txn.commit().unwrap();

        let retrieved = storage.get_kitchen_order("ko-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().order_id, "order-1");
    }
}
```

**Step 2: 更新 mod.rs**

```rust
pub mod cache;
pub mod storage;
pub mod types;

pub use cache::PrintConfigCache;
pub use storage::{PrintStorage, PrintStorageError, PrintStorageResult, PrintStorageStats};
pub use types::*;
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 4: 运行测试**

Run: `cargo test -p edge-server --lib printing`
Expected: 测试通过

**Step 5: Commit**

```bash
git add edge-server/src/printing/
git commit -m "$(cat <<'EOF'
feat(printing): implement redb storage for kitchen orders and labels

Add CRUD operations with order-based indexing and cleanup utilities.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 3: KitchenPrintService 核心实现

### Task 7: 实现 KitchenPrintService 基础框架

**Files:**
- Create: `edge-server/src/printing/service.rs`
- Modify: `edge-server/src/printing/mod.rs`

**Step 1: 创建 service.rs**

创建 `edge-server/src/printing/service.rs`：

```rust
//! Kitchen Print Service - handles automatic printing on order events

use super::{
    cache::PrintConfigCache,
    storage::PrintStorage,
    types::{KitchenOrder, KitchenOrderItem, LabelPrintRecord, PrintItemContext},
};
use shared::order::{EventPayload, OrderEvent, OrderEventType};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Debug, Error)]
pub enum PrintServiceError {
    #[error("Storage error: {0}")]
    Storage(#[from] super::storage::PrintStorageError),

    #[error("Kitchen order not found: {0}")]
    KitchenOrderNotFound(String),

    #[error("Label record not found: {0}")]
    LabelRecordNotFound(String),

    #[error("Printing disabled")]
    PrintingDisabled,
}

pub type PrintServiceResult<T> = Result<T, PrintServiceError>;

/// Kitchen Print Service
#[derive(Clone)]
pub struct KitchenPrintService {
    storage: PrintStorage,
    cache: PrintConfigCache,
    enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl KitchenPrintService {
    /// Create a new service
    pub fn new(storage: PrintStorage, cache: PrintConfigCache) -> Self {
        Self {
            storage,
            cache,
            enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    /// Enable/disable printing
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled
            .store(enabled, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the config cache (for updates)
    pub fn cache(&self) -> &PrintConfigCache {
        &self.cache
    }

    /// Get the storage (for direct queries)
    pub fn storage(&self) -> &PrintStorage {
        &self.storage
    }

    /// Handle an order event
    pub async fn handle_event(&self, event: &OrderEvent) -> PrintServiceResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        match &event.event_type {
            OrderEventType::ItemsAdded => {
                self.handle_items_added(event).await?;
            }
            // KitchenOrder/LabelPrintRecord 保留 3 天，不随订单关闭删除
            _ => {}
        }

        Ok(())
    }

    /// Handle ItemsAdded event
    async fn handle_items_added(&self, event: &OrderEvent) -> PrintServiceResult<()> {
        // Fast path: check if any printing is enabled
        let kitchen_enabled = self.cache.is_kitchen_print_enabled().await;
        let label_enabled = self.cache.is_label_print_enabled().await;

        if !kitchen_enabled && !label_enabled {
            debug!("Printing disabled, skipping ItemsAdded event");
            return Ok(());
        }

        let EventPayload::ItemsAdded { items } = &event.payload else {
            return Ok(());
        };

        // Build print contexts for each item
        let mut kitchen_items = Vec::new();
        let mut label_records = Vec::new();

        for item in items {
            let product_id = &item.id;

            // Get destinations from cache
            let kitchen_dests = if kitchen_enabled {
                self.cache.get_kitchen_destinations(product_id).await
            } else {
                vec![]
            };

            let label_dests = if label_enabled {
                self.cache.get_label_destinations(product_id).await
            } else {
                vec![]
            };

            // Skip if no destinations
            if kitchen_dests.is_empty() && label_dests.is_empty() {
                continue;
            }

            // Get product config for metadata
            let product_config = self.cache.get_product(product_id).await;
            let category_config = product_config
                .as_ref()
                .and_then(|p| {
                    let cat_id = p.category_id.clone();
                    // Can't use await here directly, would need to restructure
                    None::<super::types::CategoryPrintConfig>
                });

            // Build context
            let context = PrintItemContext {
                category_id: product_config
                    .as_ref()
                    .map(|p| p.category_id.clone())
                    .unwrap_or_default(),
                category_name: String::new(), // TODO: fill from category cache
                product_id: product_id.clone(),
                external_id: product_config.as_ref().and_then(|p| p.root_spec_external_id),
                kitchen_name: product_config
                    .as_ref()
                    .map(|p| p.kitchen_name.clone())
                    .unwrap_or_else(|| item.name.clone()),
                product_name: item.name.clone(),
                spec_name: item.selected_specification.as_ref().map(|s| s.name.clone()),
                quantity: item.quantity,
                index: None,
                options: item
                    .selected_options
                    .as_ref()
                    .map(|opts| opts.iter().map(|o| o.option_name.clone()).collect())
                    .unwrap_or_default(),
                note: item.note.clone(),
                kitchen_destinations: kitchen_dests.clone(),
                label_destinations: label_dests.clone(),
            };

            // Add to kitchen items if has destinations
            if !kitchen_dests.is_empty() {
                kitchen_items.push(KitchenOrderItem {
                    context: context.clone(),
                });
            }

            // Create label records for each quantity
            if !label_dests.is_empty() {
                for i in 1..=item.quantity {
                    let mut label_context = context.clone();
                    label_context.quantity = 1;
                    label_context.index = Some(format!("{}/{}", i, item.quantity));

                    label_records.push(LabelPrintRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        order_id: event.order_id.clone(),
                        kitchen_order_id: event.event_id.clone(),
                        table_name: None, // TODO: get from order snapshot
                        created_at: chrono::Utc::now().timestamp(),
                        context: label_context,
                        print_count: 0,
                    });
                }
            }
        }

        // Store kitchen order if has items
        if !kitchen_items.is_empty() {
            let kitchen_order = KitchenOrder {
                id: event.event_id.clone(),
                order_id: event.order_id.clone(),
                table_name: None, // TODO: get from order snapshot
                created_at: chrono::Utc::now().timestamp(),
                items: kitchen_items,
                print_count: 0,
            };

            let txn = self.storage.begin_write()?;
            self.storage.store_kitchen_order(&txn, &kitchen_order)?;
            txn.commit()?;

            info!(
                order_id = %event.order_id,
                kitchen_order_id = %kitchen_order.id,
                item_count = kitchen_order.items.len(),
                "Created kitchen order"
            );

            // TODO: send to printers
        }

        // Store label records
        if !label_records.is_empty() {
            let txn = self.storage.begin_write()?;
            for record in &label_records {
                self.storage.store_label_record(&txn, record)?;
            }
            txn.commit()?;

            info!(
                order_id = %event.order_id,
                label_count = label_records.len(),
                "Created label records"
            );

            // TODO: send to label printers
        }

        Ok(())
    }

    // 注意：KitchenOrder/LabelPrintRecord 不随订单关闭删除
    // 保留 3 天，通过 cleanup_old_records 定时清理

    // ========== API Methods ==========

    /// Get kitchen orders for an order
    pub fn get_kitchen_orders_for_order(
        &self,
        order_id: &str,
    ) -> PrintServiceResult<Vec<KitchenOrder>> {
        Ok(self.storage.get_kitchen_orders_for_order(order_id)?)
    }

    /// Get all kitchen orders (paginated)
    pub fn get_all_kitchen_orders(
        &self,
        page: usize,
        limit: usize,
    ) -> PrintServiceResult<Vec<KitchenOrder>> {
        let offset = page.saturating_sub(1) * limit;
        Ok(self.storage.get_all_kitchen_orders(offset, limit)?)
    }

    /// Reprint a kitchen order
    pub fn reprint_kitchen_order(&self, id: &str) -> PrintServiceResult<KitchenOrder> {
        let order = self
            .storage
            .get_kitchen_order(id)?
            .ok_or_else(|| PrintServiceError::KitchenOrderNotFound(id.to_string()))?;

        // Increment print count
        let txn = self.storage.begin_write()?;
        self.storage.increment_kitchen_order_print_count(&txn, id)?;
        txn.commit()?;

        // TODO: send to printers

        info!(kitchen_order_id = %id, "Reprinted kitchen order");

        // Return updated order
        Ok(self.storage.get_kitchen_order(id)?.unwrap())
    }

    /// Get label records for an order
    pub fn get_label_records_for_order(
        &self,
        order_id: &str,
    ) -> PrintServiceResult<Vec<LabelPrintRecord>> {
        Ok(self.storage.get_label_records_for_order(order_id)?)
    }

    /// Reprint a label record
    pub fn reprint_label_record(&self, id: &str) -> PrintServiceResult<LabelPrintRecord> {
        let record = self
            .storage
            .get_label_record(id)?
            .ok_or_else(|| PrintServiceError::LabelRecordNotFound(id.to_string()))?;

        let txn = self.storage.begin_write()?;
        self.storage.increment_label_record_print_count(&txn, id)?;
        txn.commit()?;

        // TODO: send to label printer

        info!(label_record_id = %id, "Reprinted label record");

        Ok(self.storage.get_label_record(id)?.unwrap())
    }

    /// Refresh config cache from database
    pub async fn refresh_cache(&self) -> PrintServiceResult<()> {
        // TODO: load from SurrealDB
        info!("Refreshed print config cache");
        Ok(())
    }

    /// Cleanup old records
    pub fn cleanup_old_records(&self, max_age_hours: u32) -> PrintServiceResult<usize> {
        let max_age_secs = (max_age_hours as i64) * 3600;
        let deleted = self.storage.cleanup_old_records(max_age_secs)?;
        if deleted > 0 {
            info!(deleted_count = deleted, "Cleaned up old print records");
        }
        Ok(deleted)
    }
}
```

**Step 2: 更新 mod.rs**

```rust
pub mod cache;
pub mod service;
pub mod storage;
pub mod types;

pub use cache::PrintConfigCache;
pub use service::{KitchenPrintService, PrintServiceError, PrintServiceResult};
pub use storage::{PrintStorage, PrintStorageError, PrintStorageResult, PrintStorageStats};
pub use types::*;
```

**Step 3: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 4: Commit**

```bash
git add edge-server/src/printing/
git commit -m "$(cat <<'EOF'
feat(printing): implement KitchenPrintService core

Handle ItemsAdded events, store records, support reprint and cleanup.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 4: API 端点

### Task 8: 实现厨房打印 API 端点

**Files:**
- Create: `edge-server/src/api/kitchen_print/mod.rs`
- Create: `edge-server/src/api/kitchen_print/handler.rs`
- Modify: `edge-server/src/api/mod.rs`

**Step 1: 创建 handler.rs**

创建 `edge-server/src/api/kitchen_print/handler.rs`：

```rust
//! Kitchen print API handlers

use crate::printing::{KitchenOrder, KitchenPrintService, LabelPrintRecord};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::utils::error::AppError;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub order_id: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct KitchenOrdersResponse {
    pub data: Vec<KitchenOrder>,
}

#[derive(Debug, Serialize)]
pub struct LabelRecordsResponse {
    pub data: Vec<LabelPrintRecord>,
}

/// GET /kitchen-orders
pub async fn list_kitchen_orders(
    State(service): State<KitchenPrintService>,
    Query(query): Query<ListQuery>,
) -> Result<Json<KitchenOrdersResponse>, AppError> {
    let orders = if let Some(order_id) = query.order_id {
        service.get_kitchen_orders_for_order(&order_id)?
    } else {
        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(20);
        service.get_all_kitchen_orders(page, limit)?
    };

    Ok(Json(KitchenOrdersResponse { data: orders }))
}

/// POST /kitchen-orders/:id/reprint
pub async fn reprint_kitchen_order(
    State(service): State<KitchenPrintService>,
    Path(id): Path<String>,
) -> Result<Json<KitchenOrder>, AppError> {
    let order = service.reprint_kitchen_order(&id)?;
    Ok(Json(order))
}

/// GET /label-records
pub async fn list_label_records(
    State(service): State<KitchenPrintService>,
    Query(query): Query<ListQuery>,
) -> Result<Json<LabelRecordsResponse>, AppError> {
    let order_id = query
        .order_id
        .ok_or_else(|| AppError::BadRequest("order_id is required".to_string()))?;
    let records = service.get_label_records_for_order(&order_id)?;
    Ok(Json(LabelRecordsResponse { data: records }))
}

/// POST /label-records/:id/reprint
pub async fn reprint_label_record(
    State(service): State<KitchenPrintService>,
    Path(id): Path<String>,
) -> Result<Json<LabelPrintRecord>, AppError> {
    let record = service.reprint_label_record(&id)?;
    Ok(Json(record))
}

/// POST /kitchen-print/refresh-cache
pub async fn refresh_cache(
    State(service): State<KitchenPrintService>,
) -> Result<Json<serde_json::Value>, AppError> {
    service.refresh_cache().await?;
    Ok(Json(serde_json::json!({ "success": true })))
}
```

**Step 2: 创建 mod.rs**

创建 `edge-server/src/api/kitchen_print/mod.rs`：

```rust
//! Kitchen print API module

pub mod handler;

use axum::{routing::{get, post}, Router};
use crate::printing::KitchenPrintService;

pub fn routes(service: KitchenPrintService) -> Router {
    Router::new()
        .route("/kitchen-orders", get(handler::list_kitchen_orders))
        .route("/kitchen-orders/:id/reprint", post(handler::reprint_kitchen_order))
        .route("/label-records", get(handler::list_label_records))
        .route("/label-records/:id/reprint", post(handler::reprint_label_record))
        .route("/kitchen-print/refresh-cache", post(handler::refresh_cache))
        .with_state(service)
}
```

**Step 3: 在 api/mod.rs 中注册路由**

在 `edge-server/src/api/mod.rs` 添加：

```rust
pub mod kitchen_print;
```

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 5: Commit**

```bash
git add edge-server/src/api/kitchen_print/
git commit -m "$(cat <<'EOF'
feat(api): add kitchen print and label record endpoints

- GET /kitchen-orders
- POST /kitchen-orders/:id/reprint
- GET /label-records
- POST /label-records/:id/reprint
- POST /kitchen-print/refresh-cache

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 5: 集成到 ServerState

### Task 9: 集成 KitchenPrintService 到 ServerState

**Files:**
- Modify: `edge-server/src/core/state.rs`
- Modify: `edge-server/src/core/server.rs`

**Step 1: 修改 ServerState**

在 `edge-server/src/core/state.rs` 中添加 `KitchenPrintService`：

```rust
use crate::printing::{KitchenPrintService, PrintConfigCache, PrintStorage};

pub struct ServerState {
    // ... existing fields
    pub kitchen_print_service: KitchenPrintService,
}
```

**Step 2: 初始化服务**

在服务器启动时初始化 `KitchenPrintService`：

```rust
// 在 ServerState::new 或初始化逻辑中
let print_storage = PrintStorage::open(&data_dir.join("print.redb"))?;
let print_cache = PrintConfigCache::new();
let kitchen_print_service = KitchenPrintService::new(print_storage, print_cache);
```

**Step 3: 注册 API 路由**

在路由配置中添加：

```rust
.merge(kitchen_print::routes(state.kitchen_print_service.clone()))
```

**Step 4: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 5: Commit**

```bash
git add edge-server/src/core/
git commit -m "$(cat <<'EOF'
feat(core): integrate KitchenPrintService into ServerState

Initialize print storage and cache on startup, register API routes.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: 集成事件监听到 OrdersManager

**Files:**
- Modify: `edge-server/src/orders/manager.rs`

**Step 1: 添加事件广播钩子**

在 `OrdersManager` 中，当事件生成后调用 `KitchenPrintService::handle_event`：

```rust
// 在事件广播后
if let Some(print_service) = &self.kitchen_print_service {
    if let Err(e) = print_service.handle_event(&event).await {
        tracing::warn!(error = %e, "Failed to handle print event");
    }
}
```

**Step 2: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 3: Commit**

```bash
git add edge-server/src/orders/manager.rs
git commit -m "$(cat <<'EOF'
feat(orders): integrate kitchen print service event handling

Call KitchenPrintService::handle_event after order event broadcast.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 6: 前端 API 客户端

### Task 11: 添加前端 API 类型和客户端

**Files:**
- Create: `red_coral/src/infrastructure/api/kitchenPrint.ts`
- Modify: `red_coral/src/infrastructure/api/index.ts`

**Step 1: 创建 kitchenPrint.ts**

```typescript
/**
 * Kitchen Print API client
 */

import { invoke } from '@tauri-apps/api/core';

// Types (match Rust backend)
export interface PrintItemContext {
  category_id: string;
  category_name: string;
  product_id: string;
  external_id: number | null;
  kitchen_name: string;
  product_name: string;
  spec_name: string | null;
  quantity: number;
  index: string | null;
  options: string[];
  note: string | null;
  kitchen_destinations: string[];
  label_destinations: string[];
}

export interface KitchenOrderItem {
  context: PrintItemContext;
}

export interface KitchenOrder {
  id: string;
  order_id: string;
  table_name: string | null;
  created_at: number;
  items: KitchenOrderItem[];
  print_count: number;
}

export interface LabelPrintRecord {
  id: string;
  order_id: string;
  kitchen_order_id: string;
  table_name: string | null;
  created_at: number;
  context: PrintItemContext;
  print_count: number;
}

export interface KitchenOrdersResponse {
  data: KitchenOrder[];
}

export interface LabelRecordsResponse {
  data: LabelPrintRecord[];
}

// API functions
export async function listKitchenOrders(params?: {
  order_id?: string;
  page?: number;
  limit?: number;
}): Promise<KitchenOrder[]> {
  const response = await invoke<KitchenOrdersResponse>('api_request', {
    method: 'GET',
    path: '/kitchen-orders',
    query: params,
  });
  return response.data;
}

export async function reprintKitchenOrder(id: string): Promise<KitchenOrder> {
  return invoke<KitchenOrder>('api_request', {
    method: 'POST',
    path: `/kitchen-orders/${id}/reprint`,
  });
}

export async function listLabelRecords(orderId: string): Promise<LabelPrintRecord[]> {
  const response = await invoke<LabelRecordsResponse>('api_request', {
    method: 'GET',
    path: '/label-records',
    query: { order_id: orderId },
  });
  return response.data;
}

export async function reprintLabelRecord(id: string): Promise<LabelPrintRecord> {
  return invoke<LabelPrintRecord>('api_request', {
    method: 'POST',
    path: `/label-records/${id}/reprint`,
  });
}

export async function refreshPrintCache(): Promise<void> {
  await invoke('api_request', {
    method: 'POST',
    path: '/kitchen-print/refresh-cache',
  });
}
```

**Step 2: 导出**

在 `red_coral/src/infrastructure/api/index.ts` 添加导出：

```typescript
export * from './kitchenPrint';
```

**Step 3: 验证 TypeScript 编译**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 编译成功

**Step 4: Commit**

```bash
git add red_coral/src/infrastructure/api/
git commit -m "$(cat <<'EOF'
feat(frontend): add kitchen print API client

Types and functions for kitchen orders and label records.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 7: 数据库迁移（如需要）

### Task 12: 更新 SurrealDB Schema (如有变化)

如果 Category/Product 字段重命名导致数据库 schema 变化：

**Files:**
- Modify: `edge-server/src/db/repository/category.rs` (如有)
- Modify: `edge-server/src/db/repository/product.rs` (如有)

**Step 1: 更新查询语句**

将 `print_destinations` 改为 `kitchen_print_destinations`。

**Step 2: 验证编译**

Run: `cargo check -p edge-server`
Expected: 编译成功

**Step 3: Commit**

```bash
git add edge-server/src/db/
git commit -m "$(cat <<'EOF'
fix(db): update queries for renamed print destination fields

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## 测试验证清单

完成以上任务后，执行以下验证：

1. **Rust 编译**: `cargo check --workspace`
2. **Rust 测试**: `cargo test --workspace --lib`
3. **Clippy 检查**: `cargo clippy --workspace`
4. **TypeScript 编译**: `cd red_coral && npx tsc --noEmit`

---

## 后续任务（不在本计划范围）

以下任务可在基础功能完成后迭代：

1. ESC/POS 厨房单渲染
2. 网络打印机连接池
3. 前端厨房小票列表页
4. 前端设置页打印配置 UI
5. 标签模板渲染
6. 系统默认打印机配置 API
