# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Red Coral - Tauri POS 前端

基于 React 19 + Zustand + Tailwind CSS 的桌面 POS 应用，通过 Tauri 与 edge-server 集成。

## 命令

```bash
npm run tauri:dev       # Tauri 开发模式
npm run dev             # Vite 开发服务器 (仅前端)
npx tsc --noEmit        # TypeScript 类型检查
npm run build           # 生产构建
npm run test            # Vitest 测试
npm run deadcode        # 查找未使用代码
```

## 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| React | 19 | UI 框架 |
| TypeScript | 5.8 | 类型系统 |
| Zustand | 5 | 状态管理 |
| Tailwind CSS | 4 | 样式 |
| Vite | 6 | 构建工具 |
| Tauri | 2 | 桌面框架 |
| React Router | 7 | 路由 |
| decimal.js | - | 精确金额计算 |
| recharts | - | 数据可视化 |
| zod | - | Schema 验证 |

## 前端目录结构

```
src/
├── core/                   # 核心业务逻辑
│   ├── domain/
│   │   ├── types/              # TypeScript 类型定义
│   │   │   ├── api/            # API 模型 (Product, Category 等，与 Rust 对齐)
│   │   │   ├── print/          # 打印/标签类型
│   │   │   ├── orderEvent.ts   # 事件溯源类型 (Command, Event, Snapshot)
│   │   │   └── appState.ts     # 应用状态机
│   │   └── validators.ts       # Zod 校验
│   ├── hooks/                  # 核心逻辑 hooks
│   │   ├── useOrderEventListener.ts  # 订单事件监听
│   │   ├── useSyncListener.ts        # 资源实时同步
│   │   ├── useConnectionStatus.ts    # 连接状态
│   │   ├── useSyncConnection.ts      # 重连同步
│   │   ├── useShiftCloseGuard.ts     # 班次关闭守卫
│   │   └── useCommandLock.ts         # 命令悲观锁
│   ├── stores/                 # Zustand 状态管理
│   │   ├── auth/               # 认证状态
│   │   ├── bridge/             # Tauri Bridge 状态 (应用生命周期)
│   │   ├── order/              # 订单领域 stores
│   │   │   ├── useActiveOrdersStore.ts   # 活跃订单 (事件溯源)
│   │   │   ├── useDraftOrderStore.ts     # 草稿订单
│   │   │   ├── useCheckoutStore.ts       # 结账状态
│   │   │   ├── useOrderOperations.ts     # 业务逻辑
│   │   │   └── useOrderCommands.ts       # 命令构建器
│   │   ├── cart/               # 购物车
│   │   ├── ui/                 # UI 状态 (侧栏, 弹窗, 屏幕)
│   │   ├── shift/              # 班次管理
│   │   ├── settings/           # 应用设置
│   │   ├── resources/          # 资源 Store 注册表 (同步资源)
│   │   └── factory/            # createResourceStore() 工厂
│   └── services/               # 业务服务
│       ├── order/paymentService.ts  # 支付处理
│       └── imageCache.ts            # 图片缓存
├── hooks/                  # 页面级 hooks
│   ├── useConfirm.ts          # 确认弹窗
│   ├── useDraftHandlers.ts    # 草稿操作
│   ├── useFormInitialization.ts # 表单初始化
│   ├── useHistoryOrderDetail.ts # 历史订单详情
│   ├── useHistoryOrderList.ts   # 历史订单列表
│   ├── useI18n.ts             # 国际化
│   ├── useLongPress.ts        # 长按手势
│   ├── useOrderHandlers.ts    # 订单操作
│   ├── usePermission.ts       # 权限查询
│   ├── usePriceInput.ts       # 价格输入
│   └── useRetailOrderRecovery.ts # 零售订单恢复
├── features/               # 功能模块 (插件式组织)
│   ├── product/            # 商品管理
│   ├── category/           # 分类管理
│   ├── attribute/          # 属性管理
│   ├── tag/                # 标签管理
│   ├── zone/               # 区域管理
│   ├── table/              # 餐桌管理
│   ├── user/               # 员工管理
│   ├── role/               # 角色权限
│   ├── price-rule/         # 价格规则
│   ├── shift/              # 班次操作
│   └── daily-report/       # 日报
├── screens/                # 全屏页面
│   ├── POS/                # 主 POS 界面 (商品网格 + 购物车 + 订单列表)
│   ├── Login/              # 员工登录
│   ├── Setup/              # 模式选择 (Server / Client)
│   ├── Activate/           # 设备激活
│   ├── Statistics/         # 销售统计 (Dashboard, SalesReport, DailyReport)
│   ├── Settings/           # 管理设置 (商品、分类、打印机、语言等)
│   ├── History/            # 订单历史 (归档查看 + Timeline)
│   ├── TableSelection/     # 桌台/客数选择
│   ├── Checkout/           # 结账支付 (多支付方式, 拆单, components/, payment/)
│   ├── TenantSelect/       # 多租户选择
│   ├── Status/             # 状态页 (ActivationRequired, SubscriptionBlocked)
│   └── Debug/              # 调试页面
├── presentation/           # 可复用 UI 组件
│   └── components/
│       ├── auth/           # 权限组件 (PermissionGate, SupervisorAuth)
│       ├── cart/           # 购物车组件
│       ├── shift/          # 班次组件
│       ├── modals/         # 弹窗 (商品选项, 订单详情等)
│       ├── notifications/  # 通知
│       └── ui/             # 基础 UI (Numpad, IconBtn 等)
├── shared/                 # 共享组件
│   └── components/
│       ├── DataTable/          # 通用数据表格
│       ├── FormField/          # 表单字段 (含 SelectField, KitchenPrinterSelector)
│       ├── Timeline/           # 订单事件时间线
│       ├── ConfirmDialog/      # 确认对话框
│       ├── DeleteConfirmation/ # 删除确认
│       ├── FilterBar/          # 过滤栏
│       └── GroupedOptionsList.tsx # 分组选项列表
├── infrastructure/         # 基础设施
│   ├── api/tauri-client.ts # Tauri 命令调用层
│   ├── i18n/               # 国际化 (zh-CN)
│   ├── print/              # 打印服务
│   └── label/              # 标签打印服务
├── utils/                  # 工具函数
│   ├── currency/           # Currency 类 (decimal.js 封装)
│   ├── error/              # friendlyError (API错误码→用户消息) + commandErrorMessage (订单命令错误码→用户消息)
│   └── formatting/         # thingId 处理等
└── generated/              # 生成代码
    └── error-codes.ts      # 从 Rust ErrorCode 生成
```

## Tauri 后端结构

```
src-tauri/src/
├── core/
│   ├── bridge/             # ClientBridge (Server/Client 双模式)
│   │   ├── mod.rs          # 结构体定义 + 构造函数
│   │   ├── lifecycle.rs    # 模式生命周期 (启动/停止/恢复/重建)
│   │   ├── order_es.rs     # 订单事件溯源 API
│   │   ├── api.rs          # 通用 API 透传
│   │   ├── auth.rs         # 员工认证
│   │   ├── activation.rs   # 设备激活
│   │   ├── state.rs        # 状态查询 (AppState)
│   │   ├── types.rs        # AppState 枚举
│   │   └── config.rs       # Server/Client 配置
│   ├── response.rs         # ApiResponse 类型定义
│   ├── tenant_manager.rs   # 多租户证书/会话管理
│   ├── session_cache.rs    # 员工会话缓存 (离线登录)
│   ├── image_cache.rs      # 图片缓存
│   └── paths.rs            # 数据路径工具
├── commands/               # Tauri 命令处理器 (16 模块)
│   ├── auth.rs             # 员工登录/登出
│   ├── order_es.rs         # 事件溯源命令
│   ├── orders.rs           # 订单查询 (归档历史)
│   ├── sync.rs             # 实时同步订阅
│   ├── shift.rs            # 班次操作
│   ├── data.rs             # 资源 CRUD
│   ├── statistics.rs       # 统计分析
│   ├── mode.rs             # Server/Client 模式切换
│   ├── tenant.rs           # 租户管理
│   ├── health.rs           # 健康检查
│   ├── image.rs            # 图片上传/管理
│   ├── printer.rs          # 打印机管理
│   ├── location.rs         # 定位服务
│   ├── backup.rs           # 备份/恢复
│   ├── system.rs           # 系统操作
│   └── api.rs              # 通用 API 透传
└── utils/
    ├── receipt_renderer.rs # 收据渲染
    └── printing.rs         # 打印分发
```

## 核心架构模式

### 状态管理 (三层 Store)

1. **领域 Store**: useActiveOrdersStore, useAuthStore, useCheckoutStore (业务逻辑)
2. **资源 Store**: `createResourceStore()` 工厂创建，自动同步 (Products, Categories 等)
3. **UI Store**: useUIStore, useSettingsStore (界面状态)

### 资源同步流程

```
Server 变更 → MessageBus broadcast
  → Tauri event: resource-sync:<resource>
  → useSyncListener 监听
  → storeRegistry 分发到对应 Store
  → 版本号对比 → 增量/全量更新
  → React 组件自动 re-render
```

### 应用状态机

```
Uninitialized → Setup → Activate → ServerReady → Login → Authenticated (POS)
                  ↓
           ClientNeedSetup → ClientConnecting → ClientConnected → Login → Authenticated
```

关键状态: ServerSubscriptionBlocked (订阅过期/无效)

### 权限模型

- **PermissionGate**: 组件级权限守卫
- **SupervisorAuthModal**: 权限升级 (主管重新认证)
- **usePermission()**: 权限查询 hook
- 格式: `resource:action` (如 `orders:void`, `products:write`)

### 订单事件溯源 (前端)

- **服务端权威**: 前端不做本地计算，所有状态来自服务端
- **命令模式**: useOrderCommands 构建 OrderCommand → Tauri invoke → edge-server
- **事件监听**: useOrderEventListener 接收服务端事件 → 更新 Store

## Feature 模块规范

每个 feature 模块遵循统一结构:
```
features/<name>/
├── <Name>Management.tsx    # 主管理页面
├── <Name>Modal.tsx         # 创建/编辑弹窗
├── <Name>Form.tsx          # 表单组件
├── mutations.ts            # API 调用
├── store.ts                # Zustand Store 实例
└── index.ts                # 导出
```

## 关键约束

跨前后端通用约定（金额、时间戳、类型对齐、Tauri 命令参数等）见 [`docs/claude/conventions.md`](../docs/claude/conventions.md)。

以下为 red_coral 前端专属约束：

| 约束 | 说明 |
|------|------|
| **货币格式** | 使用 `formatCurrency()` 统一格式化，禁止硬编码货币符号 |
| **snake_case** | 表单字段和 API 数据统一使用 snake_case |
| **Tauri 命令参数** | `invokeApi` 顶层参数用 camelCase（如 `sinceSequence`, `orderId`）；**嵌套 struct/数组内的字段由 serde 反序列化，前端必须手动转为 snake_case 发送**（如 `LabelField.dataSource` → `data_source`），接收时手动转回 camelCase |
| **懒加载** | Tauri Client 使用懒加载，禁止模块作用域直接创建 |
| **错误处理** | 关键加载失败使用 dialog 阻断，禁止静默错误 |
| **Zustand Selector** | selector 必须返回稳定引用；需要派生数据时先选原始数据再用 `useMemo`，禁止在 selector 内 `new Map()`/`.filter()`/`.map()` |

## 颜色语言 (价格明细)

全局统一的颜色分配，所有涉及价格明细的组件必须遵守:

| 类型 | 文字颜色 | 徽标颜色 | 按钮颜色 |
|------|----------|----------|----------|
| **赠送 (comp)** | `text-emerald-600` | - | `bg-emerald-500` |
| **手动折扣 (manual discount)** | `text-orange-500` | `bg-orange-100 text-orange-700` | `bg-orange-500` |
| **规则折扣 (rule discount)** | `text-amber-600` | `bg-amber-100 text-amber-700` | - |
| **规则附加费 (rule surcharge)** | `text-purple-500` | `bg-purple-100 text-purple-700` | - |
| **整单折扣 (order discount)** | `text-orange-500` | - | `bg-orange-500` |
| **整单附加费 (order surcharge)** | `text-purple-500` | - | `bg-purple-500` |

**适用组件**: OrderSidebar, OrderDetailMode, HistoryDetail, PaymentFlow, CartItem, UnpaidItemRow, PriceRuleManagement, ItemActionPanel

## 响应语言

使用中文回答。
