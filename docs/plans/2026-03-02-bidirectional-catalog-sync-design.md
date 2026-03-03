# 双向 Catalog 同步设计

## 问题

客户换设备/重装系统后 re-bind store，Edge SQLite 为空，Cloud PG 有 57 个商品。
当前系统没有 Cloud→Edge 全量同步，也没有 Edge→Cloud catalog 上行同步。

## 约束

1. **re-bind = 断链**: 新设备 chain_entry 从 0 开始，老订单已在 Cloud，不需要同步
2. **re-bind 是 RedCoral 主动发起的**: 不需要检测 Edge 空不空，re-bind 事件本身就是触发器
3. **双向权威**: Edge 和 Cloud 都可以修改 catalog，任何一方不能覆盖另一方
4. **离线场景**: Edge 离线期间双方都可能修改 catalog，上线后需要合并

## 同步范围

| 资源 | re-bind 后同步 | 方向 |
|------|-------------|------|
| Catalog (product/category/attribute/tag) | 需要 | 双向 |
| Resource (employee/zone/table/price_rule/label_template) | 需要 | 双向 |
| StoreInfo | 需要 | 双向 |
| archived_order / credit_note / invoice | 不需要 | 老的已在 Cloud |
| chain_entry | 新链从 0 开始 | 只上行新的 |

## 设计

### Schema 变更

所有 catalog/resource 表加 `updated_at INTEGER NOT NULL DEFAULT 0`：

**Edge SQLite** (migration):
- product, category, tag, attribute, attribute_option
- attribute_binding, product_spec, product_tag, category_tag
- category_print_destination
- employee, zone, dining_table, price_rule, label_template
- store_info (在 system_state 中)

**Cloud PG** (已有 `updated_at` 在大部分 `store_*` 表中，需确认补齐)

所有写入操作（insert/update）时设置 `updated_at = now_millis()`。

### 删除策略

保持物理删除 + catalog_changelog 记录删除事件。

catalog_changelog 表记录所有变更（包括删除），对端根据 changelog 执行对应操作。

### Edge 侧：catalog_changelog 表

```sql
CREATE TABLE catalog_changelog (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource TEXT NOT NULL,       -- 'product', 'category', 'tag', ...
    resource_id INTEGER NOT NULL, -- 被修改的资源 ID
    action TEXT NOT NULL,         -- 'upsert' | 'delete'
    data TEXT,                    -- JSON snapshot (upsert 时), NULL (delete 时)
    updated_at INTEGER NOT NULL,  -- 变更时间戳
    cloud_synced INTEGER NOT NULL DEFAULT 0  -- 0=pending, 1=synced
);
```

**写入时机**: CatalogService 的每次 create/update/delete 操作追加一条记录。
Cloud 下推（`cloud_origin=true`）的变更 **不写 changelog**（避免回弹）。

### Cloud 侧：pending_ops（已有）

Cloud Console 编辑产生的 StoreOp 通过现有 `pending_ops` 队列在 Edge 离线时排队。
这部分不变。

### 协议变更

#### 新增 CloudMessage variant

```rust
// Edge → Cloud: 请求全量 catalog（re-bind 后发送）
CloudMessage::RequestCatalogSync

// Cloud → Edge: 全量 catalog 响应（带 ID + updated_at）
CloudMessage::CatalogSyncData {
    items: Vec<CatalogSyncItem>,
}
```

#### CatalogSyncItem

```rust
pub struct CatalogSyncItem {
    pub resource: SyncResource,     // Product, Category, Tag, ...
    pub resource_id: i64,
    pub action: SyncAction,         // Upsert | Delete
    pub data: serde_json::Value,    // 完整 JSON (含 updated_at)
    pub updated_at: i64,
}
```

复用现有 `CloudSyncItem` 结构即可（已有 resource, resource_id, action, data 字段），只需确保 data 中包含 `updated_at`。

### 流程

#### 1. re-bind 触发的全量同步

```
RedCoral: re-bind store (activate_device with store_id)
  → 启动 edge-server (空 SQLite)
  → CloudSyncWorker 连接 WS
  → Cloud 发 Welcome { cursors }
  → Edge 检查: 所有 cursor 为 0 (空库)
  → Edge 发 RequestCatalogSync
  → Cloud 收到后:
      查询该 store_id 的所有 catalog 资源 (store_products, store_categories, ...)
      构建 CatalogSyncItem[] (每个 item 带完整数据 + source_id 作为 ID)
      发 CatalogSyncData { items }
  → Edge 收到后:
      对每个 item 执行 UPSERT (INSERT OR REPLACE)
      使用 source_id 作为 assigned_id
      catalog_changelog 不记录 (这是从 Cloud 来的)
      warmup() 刷新内存缓存
```

#### 2. 正常重连（非 re-bind）的增量同步

**Cloud → Edge**: 现有 pending_ops 回放（不变）

**Edge → Cloud**: CloudSyncWorker 在连接建立后，扫描 `catalog_changelog WHERE cloud_synced = 0` 发送 SyncBatch。

```
CloudSyncWorker::run_ws_session():
  1. wait_for_welcome(cursors)
  2. send_initial_sync(cursors)           // 现有：资源版本同步
  3. send_catalog_changelog(&mut ws_sink) // 新增：catalog 变更上行
  4. sync_archives_http("catch-up")       // 现有：订单归档
  ...
```

#### 3. 在线实时同步

**Cloud → Edge**: 现有 StoreOp 推送（不变）。Edge 执行后 **不写 changelog**（cloud_origin=true）。

**Edge → Cloud**: Edge 本地修改触发 broadcast_sync → CloudSyncWorker debounce → SyncBatch 上行（现有机制，需扩展支持 catalog 资源）。

#### 4. 冲突解决：LWW

双方 UPSERT 时检查 `updated_at`:
- 如果 incoming.updated_at > local.updated_at → 执行更新
- 如果 incoming.updated_at <= local.updated_at → 跳过（本地更新）

Edge 侧 `rpc_executor.rs` 已有 LWW guard（`lww_check`），这个逻辑保持不变。
Cloud 侧 `upsert_resource` 需要加同样的 LWW check。

### 实现阶段

#### Phase 1: Schema + updated_at
- Edge SQLite migration: 所有 catalog 表加 `updated_at`
- 所有写入操作维护 `updated_at = now_millis()`
- Cloud 确认 PG 表的 `updated_at` 完整

#### Phase 2: catalog_changelog + 上行同步
- 新建 `catalog_changelog` 表
- CatalogService 写操作追加 changelog（非 cloud_origin 时）
- CloudSyncWorker 扫描 changelog 发送 SyncBatch
- Cloud 侧 `upsert_resource` 处理 catalog 资源 + LWW

#### Phase 3: RequestCatalogSync (re-bind 全量)
- 新增 `CloudMessage::RequestCatalogSync`
- Cloud 响应: 查询全量 catalog → 发送
- Edge 接收: UPSERT 所有 items → warmup

#### Phase 4: pending_ops LWW 增强
- Cloud pending_ops 回放时 Edge 的 LWW guard 已有
- Cloud 侧接收 Edge SyncBatch 时加 LWW guard
