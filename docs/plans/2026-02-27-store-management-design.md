# 门店管理设计

## 背景

当前门店（store）由设备激活自动创建，无法在 Console 中管理。客户电脑坏了换电脑时，只能在配额满时才能触发替换逻辑。需要：
1. Console 中完整的门店管理能力（查看设备、删除门店）
2. RedCoral Setup 时始终可选"新建门店"或"替换已有门店设备"
3. 移除 Client 数量限制，只保留门店数量限制

## 数据模型变更

### `stores` 表新增字段

```sql
ALTER TABLE stores ADD COLUMN status TEXT NOT NULL DEFAULT 'active';  -- active / deleted
ALTER TABLE stores ADD COLUMN deleted_at BIGINT;  -- 软删除时间戳，30 天后可彻底清理
```

### `subscriptions` 表

- `max_edge_servers` → 重命名为 `max_stores`
- 移除 `max_clients`

### `activations` / `client_connections`

不变。记录永久保留用于审计取证。状态照常流转（active/deactivated/replaced）。

## Console 门店管理

### 门店列表页 (StoresScreen)

- 只显示 `status = 'active'` 的门店（现有行为不变）

### 门店设置页 (StoreSettingsScreen) 新增

**设备区域：**
- 显示关联的 Server activation（当前）+ 所有 Client connections
- 含历史记录（replaced/deactivated 状态灰显）
- 只读，用于审计

**删除门店：**
- 底部危险区域，二次确认弹窗
- 执行：软删除门店 + 停用所有关联设备（Server + Clients）
- 配额立即释放

## RedCoral Setup 改造

### Server 模式

现有流程：credentials → mode → configure → complete

改造后：credentials → mode → **选择门店** → configure → complete

「选择门店」步骤：
- 列出租户所有 `active` 门店 + 一个「新建门店」选项
- 选择已有门店 = 替换该门店的 Server 设备（旧 activation 标记 replaced）
- 选择新建 = 当前行为，创建新 store + activation
- 始终显示，不再仅在配额满时触发

### Client 模式

- 移除配额检查，直接激活
- 不再有 "quota full" 替换逻辑

## Cloud API 变更

| 方法 | 路径 | 用途 |
|------|------|------|
| GET | `/api/tenant/stores/{id}/devices` | 获取门店关联的所有设备（Server + Clients） |
| DELETE | `/api/tenant/stores/{id}` | 软删除门店 + 停用所有关联设备 |
| POST | `/api/server/activate` | 新增可选 `store_id` 参数：指定 = 替换该门店设备，不指定 = 新建 |
| POST | `/api/client/activate` | 移除配额检查 |

### 激活 API 改造

`POST /api/server/activate` 请求体新增：
- `store_id: Option<i64>` — 指定时替换该门店的 Server 设备，不指定时新建门店
- 移除 `replace_entity_id`（不再需要，通过 `store_id` 隐式确定要替换的设备）

### 配额逻辑

- 只计算 `stores WHERE status = 'active'` 的数量 vs `max_stores`
- Client 激活无配额限制

## 30 天清理

Cloud 端定时任务，清理 `stores WHERE status = 'deleted' AND deleted_at < now - 30d`：
- 级联删除所有 `store_*` 关联数据（store_products, store_categories 等）
- `activations` / `client_connections` 记录不删除（审计保留）

## 不在范围内

- 门店创建的 Console UI（门店仍由 Setup 激活创建）
- Client 终端的独立管理页面
- 已删除门店的恢复功能
