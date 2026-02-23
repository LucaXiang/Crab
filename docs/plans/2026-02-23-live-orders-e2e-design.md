# 活跃订单实时推送 — 端到端交付设计

## 概述

将已完成的后端实时推送管道（edge→cloud→console WS）延伸到前端，实现 crab-console 实时活跃订单看板。

## 当前状态

| 层 | 状态 |
|---|------|
| shared 协议 (ConsoleMessage/Command) | 完成 |
| edge-server 推送 (CloudWorker) | 完成 |
| crab-cloud LiveOrderHub + Console WS | 完成 |
| crab-console WS 客户端 | 未实现 |
| crab-console 实时订单页面 | 未实现 |
| 后端集成测试 | 未实现 |
| 部署脚本 | 缺少 |

## 隔离模型

- **租户隔离**: JWT → tenant_id → LiveOrderHub 按 tenant 隔离
- **门店隔离**: 1 edge_server = 1 门店。console 进入 `/stores/[id]/live` 时发送 `Subscribe { edge_server_ids: [id] }`，只接收该门店的订单
- **缓存隔离**: LiveOrderHub 三级嵌套 `tenant → edge_server_id → order_id`

## 交付内容

### 1. Console WebSocket 客户端

文件: `crab-console/src/lib/stores/liveOrders.ts`

- 连接 `wss://auth.redcoral.app/api/tenant/live-orders/ws`（复用 JWT）
- 自动重连（指数退避 1s→30s）
- 进入门店 live 页面时连接，离开时断开
- 连接后发送 `Subscribe { edge_server_ids: [storeId] }`
- 消息处理:
  - Ready → 替换整个 orders Map
  - OrderUpdated → upsert 单条
  - OrderRemoved → delete 单条
  - EdgeStatus { online: false, cleared_order_ids } → 删除 cleared 订单 + 标记离线
- 暴露状态: orders Map, edgeOnline boolean, connectionState

### 2. 实时订单页面

路由: `/stores/[id]/live`

- 顶部: 连接状态 + 门店名 + 活跃订单计数
- 主体: 订单卡片网格（响应式 2-3 列）
- 卡片: 订单号、桌号/服务类型、金额、商品数、时间
- Edge 离线时显示警告横幅
- 只读，不做操作

### 3. 后端集成测试

文件: `crab-cloud/tests/live_hub.rs`（纯内存，无 DB）

1. 基本流程: publish → get_all_active → remove → 验证缓存
2. 多 tenant 隔离: tenant_a 订单不出现在 tenant_b
3. 多门店隔离: Subscribe 过滤验证
4. clear_edge: 缓存清空 + EdgeOffline 携带 cleared_order_ids
5. tenant 自动清理: 无 edge + 无 subscriber → 条目移除

### 4. 部署脚本

文件: `deploy/sync-console.sh`（参考 sync-portal.sh）

### 5. 导航集成

- 门店子导航添加 "Live Orders" 入口
- i18n: nav.live_orders（EN/ZH/ES）

## 文件清单

### 新建

| 文件 | 用途 |
|------|------|
| `crab-console/src/lib/stores/liveOrders.ts` | WS 客户端 + store |
| `crab-console/src/routes/stores/[id]/live/+page.svelte` | 实时订单页面 |
| `crab-cloud/tests/live_hub.rs` | LiveOrderHub 集成测试 |
| `deploy/sync-console.sh` | Console 部署脚本 |

### 修改

| 文件 | 变更 |
|------|------|
| `crab-console/src/routes/stores/[id]/+page.svelte` | subNav 添加 Live Orders |
| `crab-console/src/lib/translations/{en,zh,es}.ts` | nav.live_orders 翻译 |
