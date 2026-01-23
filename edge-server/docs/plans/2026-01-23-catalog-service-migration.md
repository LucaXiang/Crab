# CatalogService 迁移计划

## 迁移步骤

### Step 1: 创建 CatalogService
- [ ] 新建 `services/catalog_service.rs`
- [ ] 实现核心结构体和类型 (ProductMeta, KitchenPrintConfig, LabelPrintConfig)
- [ ] 实现 warmup (加载 products + categories + tags + attributes)
- [ ] 实现 Product CRUD (写DB→更新缓存)
- [ ] 实现 Category CRUD (写DB→更新缓存)
- [ ] 实现便捷方法 (get_product_meta, get_kitchen_print_config, get_label_print_config)

### Step 2: 更新 ServerState
- [ ] 添加 `catalog_service: CatalogService`
- [ ] 删除 `product_repo: ProductRepository`
- [ ] 删除 `category_repo: CategoryRepository`
- [ ] 删除 `warmup_product_metadata_cache()` 调用
- [ ] 删除 `warmup_print_config_cache()` 调用
- [ ] 添加 `catalog_service.warmup()` 调用

### Step 3: 迁移 API Handlers
- [ ] `api/products/handler.rs` → 改用 CatalogService
- [ ] `api/categories/handler.rs` → 改用 CatalogService

### Step 4: 迁移 OrdersManager
- [ ] 删除 `product_meta_cache` 字段
- [ ] 删除 `ProductMeta` 结构体
- [ ] `AddItemsAction` 从 CatalogService 获取 meta

### Step 5: 迁移 PriceRuleEngine
- [ ] 删除 `apply_rules_to_item` 中的 DB 查询
- [ ] 从 CatalogService 获取 product meta

### Step 6: 迁移 KitchenPrintService
- [ ] 删除 `PrintConfigCache`
- [ ] `process_items_added` 从 CatalogService 获取打印配置

### Step 7: 清理
- [ ] 删除 `db/repository/product.rs`
- [ ] 删除 `db/repository/category.rs`
- [ ] 删除 `printing/cache.rs`
- [ ] 更新 mod.rs 导出

## 文件变更清单

| 操作 | 文件 |
|------|------|
| 新建 | `services/catalog_service.rs` |
| 修改 | `services/mod.rs` |
| 修改 | `core/state.rs` |
| 修改 | `api/products/handler.rs` |
| 修改 | `api/categories/handler.rs` |
| 修改 | `orders/manager.rs` |
| 修改 | `pricing/engine.rs` |
| 修改 | `printing/service.rs` |
| 修改 | `printing/mod.rs` |
| 删除 | `db/repository/product.rs` |
| 删除 | `db/repository/category.rs` |
| 删除 | `printing/cache.rs` |
