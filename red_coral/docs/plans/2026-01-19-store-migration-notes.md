# Store 迁移说明

## 当前状态

**新旧架构并存**：新架构已实现，但 POSScreen 等组件仍使用旧架构。Sync 信号和连接恢复同时更新新旧 stores。

## 已完成的改动

### 1. 新架构：服务器权威模型

创建了统一的资源 Store 架构，位于 `src/core/stores/resources/`：

```
src/core/stores/
├── factory/
│   └── createResourceStore.ts   # 工厂函数
├── resources/
│   ├── index.ts                 # 统一导出
│   ├── registry.ts              # Store 注册表
│   ├── useProductStore.ts       # 产品
│   ├── useCategoryStore.ts      # 分类
│   ├── useTagStore.ts           # 标签
│   ├── useAttributeStore.ts     # 属性模板
│   ├── useSpecStore.ts          # 规格
│   ├── useZoneStore.ts          # 区域
│   ├── useTableStore.ts         # 桌台
│   ├── useEmployeeStore.ts      # 员工
│   ├── useRoleStore.ts          # 角色
│   ├── usePriceRuleStore.ts     # 价格规则
│   ├── useKitchenPrinterStore.ts # 厨房打印机
│   └── useOrderStore.ts         # 订单
```

### 2. Hooks（同时支持新旧架构）

- `useSyncListener` - 监听 Sync 信号，同时调用新旧 stores 的刷新方法
- `useConnectionRecovery` - 连接恢复时刷新所有已加载的新旧 stores
- `usePreloadCoreData` - 预加载新架构核心数据（暂未使用）
- `CoreDataGate` - 预加载门控组件（暂未使用）

### 3. UI 流程

```
App 启动
  ↓
useSyncListener + useConnectionRecovery 挂载（监听器就位）
  ↓
InitialRoute 检查状态 → 自动启动 Server 模式
  ↓
用户登录 (/login)
  ↓
ProtectedRoute 检查
  ↓
POSScreen 加载
  ├─ 调用旧 stores: loadProducts(), loadCategories()
  └─ 显示 POS 界面
  ↓
后端数据变更 → 广播 Sync 信号
  ↓
useSyncListener 收到信号
  ├─ 调用新 stores: applySync()
  └─ 调用旧 stores: loadProducts(), refreshData()
  ↓
UI 自动更新（通过旧 stores 的 Zustand 订阅）
```

## 使用方式

### 新架构（推荐，待组件迁移后使用）

```typescript
// 直接从 resources 导入
import { useProducts, useCategories } from '@/core/stores/resources';

function MyComponent() {
  const products = useProducts();
  const categories = useCategories();
  // 数据变化时自动重新渲染
}
```

### 旧架构（当前使用）

```typescript
// POSScreen 等组件当前使用的方式
import { useProducts, useCategoryData } from '@/core/stores/product';

function POSScreen() {
  const products = useProducts();
  const { categories } = useCategoryData();
  // ...
}
```

## 待迁移的组件

以下组件使用旧架构，需要逐步迁移：

1. **POSScreen** (`src/screens/POS/index.tsx`)
   - 当前使用: `useProducts`, `useCategoryData` from `@/core/stores/product`
   - 迁移到: `useProducts`, `useCategories` from `@/core/stores/resources`

2. **Settings 相关组件**
   - 当前使用: `useSettingsStore`
   - 迁移到: `useZones`, `useTables` from `@/core/stores/resources`

3. **TableSelection** (`src/screens/TableSelection/`)
   - 当前使用: `useSettingsStore`
   - 迁移到: `useTables`, `useZones` from `@/core/stores/resources`

## 迁移步骤

1. 更新组件导入，使用新 stores
2. 移除组件中的手动 fetch 调用（新架构自动处理）
3. 使用 `CoreDataGate` 包装需要预加载的路由
4. 测试 Sync 信号和连接恢复功能
5. 从 hooks 中移除旧架构支持代码

## 已清理的废弃代码

旧架构曾使用增量同步方法，现已改为服务器权威的全量刷新模式，以下方法已删除：

### useProductStore.ts
- ~~`applySync(action, id, data)`~~ - 增量同步产品（已删除）
- ~~`setVersion(version)`~~ - 设置数据版本（已删除）

### useSettingsStore.ts
- ~~`applySyncZone(action, id, data)`~~ - 增量同步区域（已删除）
- ~~`applySyncTable(action, id, data)`~~ - 增量同步桌台（已删除）
- ~~`setDataVersion(version)`~~ - 设置数据版本（已删除）

现在使用 `loadProducts()`, `loadCategories()`, `refreshData()` 进行全量刷新。

## 注意事项

1. **命名冲突**：新旧架构有同名导出，需要明确指定导入路径
2. **Spec Store**：需要后端添加 `list_all_specs` 命令
3. **数据不共享**：新旧 stores 是独立的，数据不互通

## 后续工作

1. 迁移 POSScreen 使用新 Store
2. 迁移 Settings 相关组件
3. 迁移 TableSelection
4. 删除旧 Store 文件
5. 从 hooks 中移除旧架构支持
6. 启用 CoreDataGate
