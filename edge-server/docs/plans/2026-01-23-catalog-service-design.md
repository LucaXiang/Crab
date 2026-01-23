# CatalogService 设计

## 概述

统一管理 Product + Category 的 CRUD 和内存缓存，替代现有的：
- `ProductRepository` 直接访问
- `OrdersManager.product_meta_cache`
- `PrintConfigCache` 产品/分类部分
- `PriceRuleEngine` 数据库查询

## 核心结构

```rust
pub struct CatalogService {
    db: Surreal<Db>,
    /// 产品缓存 (ProductFull: 含 tags, attributes, specs)
    products: Arc<RwLock<HashMap<String, ProductFull>>>,
    /// 分类缓存
    categories: Arc<RwLock<HashMap<String, Category>>>,
}
```

## API

```rust
impl CatalogService {
    // ===== Product CRUD =====
    pub async fn create_product(&self, data: ProductCreate) -> Result<ProductFull>;
    pub async fn update_product(&self, id: &str, data: ProductUpdate) -> Result<ProductFull>;
    pub async fn delete_product(&self, id: &str) -> Result<()>;
    pub fn get_product(&self, id: &str) -> Option<ProductFull>;
    pub fn list_products(&self) -> Vec<&ProductFull>;
    pub fn get_products_by_category(&self, category_id: &str) -> Vec<&ProductFull>;

    // ===== Category CRUD =====
    pub async fn create_category(&self, data: CategoryCreate) -> Result<Category>;
    pub async fn update_category(&self, id: &str, data: CategoryUpdate) -> Result<Category>;
    pub async fn delete_category(&self, id: &str) -> Result<()>;
    pub fn get_category(&self, id: &str) -> Option<Category>;
    pub fn list_categories(&self) -> Vec<&Category>;

    // ===== 便捷方法（价格规则用）=====
    pub fn get_product_meta(&self, id: &str) -> Option<ProductMeta>;
    pub fn get_product_meta_batch(&self, ids: &[String]) -> HashMap<String, ProductMeta>;

    // ===== 打印配置 =====
    pub fn get_kitchen_print_config(&self, product_id: &str) -> Option<KitchenPrintConfig>;
    pub fn get_label_print_config(&self, product_id: &str) -> Option<LabelPrintConfig>;

    // ===== 启动 =====
    pub async fn warmup(&self);
}
```

## 数据结构

### ProductMeta（价格规则匹配用）
```rust
pub struct ProductMeta {
    pub category_id: String,   // "category:xxx"
    pub tags: Vec<String>,     // ["tag:xxx", ...]
}
```

### 打印配置
```rust
pub struct KitchenPrintConfig {
    pub destinations: Vec<String>,
    pub kitchen_name: Option<String>,
}

pub struct LabelPrintConfig {
    pub destinations: Vec<String>,
}
```

## 打印策略优先级

### 是否打印（启用开关）
```
全局打印策略（总开关）
  → product.is_kitchen_print_enabled (1=启用, 0=禁用, -1=继承)
  → category.is_kitchen_print_enabled (仅非虚拟分类)
```

### 打印到哪里（destinations）
```
product.kitchen_print_destinations（如果非空）
  → category.kitchen_print_destinations（如果非空，仅非虚拟分类）
  → 全局默认 destinations
```

## 关键约束

1. **产品只能属于非虚拟分类**：`product.category` 必须指向 `is_virtual = false` 的分类
2. **虚拟分类 = tags 聚合**：前端展示用，不参与打印回退
3. **写操作原子性**：DB 成功 → 更新缓存，DB 失败 → 缓存不变

## 启动预热

```rust
pub async fn warmup(&self) {
    // 1. 加载所有分类
    let categories = SELECT * FROM category;

    // 2. 加载所有产品 (FETCH tags)
    let products = SELECT * FROM product WHERE is_active = true FETCH tags;

    // 3. 批量加载属性绑定
    let bindings = SELECT * FROM attribute_binding WHERE in.is_active = true FETCH out;

    // 4. 组装 ProductFull 并存入缓存
}
```

## 迁移计划

### 删除代码
| 位置 | 删除内容 |
|------|---------|
| `orders/manager.rs` | `product_meta_cache` 及相关方法 |
| `orders/manager.rs` | `ProductMeta` 结构体 |
| `printing/cache.rs` | `ProductPrintConfig`, `CategoryPrintConfig` |
| `state.rs` | `warmup_product_metadata_cache()` |
| `state.rs` | `warmup_print_config_cache()` |
| `pricing/engine.rs` | DB 查询逻辑 |

### 替换调用
```
ProductRepository.find_all()       → CatalogService.list_products()
ProductRepository.find_by_id()     → CatalogService.get_product()
ProductRepository.create()         → CatalogService.create_product()
CategoryRepository.find_all()      → CatalogService.list_categories()
CategoryRepository.update()        → CatalogService.update_category()
OrdersManager.get_product_meta()   → CatalogService.get_product_meta()
PrintConfigCache.get_product()     → CatalogService.get_kitchen_print_config()
```

### ServerState 变化
```rust
// 之前
pub struct ServerState {
    pub product_repo: ProductRepository,
    pub category_repo: CategoryRepository,
    pub orders_manager: OrdersManager,  // 含 product_meta_cache
    pub kitchen_print_service: KitchenPrintService,  // 含 PrintConfigCache
}

// 之后
pub struct ServerState {
    pub catalog_service: CatalogService,  // 统一管理
    pub orders_manager: OrdersManager,     // 简化，无缓存
    pub kitchen_print_service: KitchenPrintService,  // 简化，从 CatalogService 读
}
```

## 实现步骤

1. **创建 CatalogService**
   - 新建 `edge-server/src/services/catalog_service.rs`
   - 实现 Product CRUD + 缓存
   - 实现 Category CRUD + 缓存
   - 实现 warmup

2. **迁移 API handlers**
   - `api/products/handler.rs` 改用 CatalogService
   - `api/categories/handler.rs` 改用 CatalogService

3. **迁移 OrdersManager**
   - 删除 `product_meta_cache`
   - `AddItemsAction` 从 CatalogService 获取 meta

4. **迁移 KitchenPrintService**
   - 删除 `PrintConfigCache`
   - `process_items_added` 从 CatalogService 获取打印配置

5. **清理**
   - 删除 `ProductRepository`, `CategoryRepository`（或保留但不直接使用）
   - 删除 `state.rs` 中的 warmup 逻辑
   - 删除 `pricing/engine.rs` 中的 DB 查询

6. **测试**
   - 单元测试 CatalogService
   - 集成测试打印回退链
   - 集成测试价格规则匹配
