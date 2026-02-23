# Console Catalog CRUD Design

## 概览

为 crab-console 添加完整的 catalog 管理功能，覆盖 9 种资源的 CRUD 操作。

## 架构

Console UI → REST API (`/api/tenant/stores/{id}/catalog/*`) → crab-cloud → WS RPC → edge-server → sync back → Console refresh

## 交互模式

列表 + Modal 弹窗（统一模式）

## 页面

| 路由 | 资源 |
|------|------|
| `/stores/[id]/products` | 产品 (含 specs, tags) |
| `/stores/[id]/categories` | 分类 |
| `/stores/[id]/tags` | 标签 |
| `/stores/[id]/attributes` | 属性 (含 options) |
| `/stores/[id]/price-rules` | 价格规则 |
| `/stores/[id]/employees` | 员工 |
| `/stores/[id]/zones` | 区域 |
| `/stores/[id]/tables` | 桌台 |

## 共享组件

- `CatalogModal.svelte` — Modal 容器
- `CatalogListPage.svelte` — 列表页骨架
- `DeleteConfirm.svelte` — 删除确认
- `ColorPicker.svelte` — 标签颜色
- `TagSelector.svelte` — 多选标签

## API 客户端

统一 CRUD pattern: list/create/update/delete per resource

## 实施批次

1. 基础设施: 共享组件 + API 函数 + 导航 + i18n
2. 简单资源: Tags, Zones, Employees, Tables
3. 中等资源: Categories, Products
4. 复杂资源: Attributes, Price Rules
