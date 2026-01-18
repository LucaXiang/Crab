# RedCoral POS - 前端迁移指南

> **前端版本**: 1.1.0
> **后端版本**: 1.1.0 (迁移后)
> **状态**: ✅ 迁移完成 - 已引入 Zod 数据校验

## 目录

- [概览](#概览)
- [API 变更](#api-变更)
- [价格处理](#价格处理)
- [软删除与过滤](#软删除与过滤)
- [类型映射](#类型映射)
- [状态管理](#状态管理)
- [常见问题](#常见问题)

## 概览

本次迁移将后端数据库从 `f64` (元) 改为 `i64` (分) 存储金额，引入软删除机制和哈希链保护。前端 API 交互格式不变，后端自动完成单位转换。

### 设计原则

1. **API 层**: 前端与后端 API 使用 `f64` (元) 进行金额交互
2. **数据库层**: 后端内部使用 `i64` (分) 存储，避免浮点精度问题
3. **软删除**: 业务实体新增 `is_deleted` 标记，查询时需过滤
4. **货币单位**: 所有金额使用 **欧元 (€)**

### 变更摘要

| 变更项 | 旧行为 | 新行为 |
|--------|--------|--------|
| 价格存储 | `f64` (元) | `i64` (分) |
| 价格精度 | 可能有精度问题 | 完全精确 |
| 删除操作 | 硬删除 | 软删除 (`is_deleted=1`) |
| 订单哈希 | 无 | SHA256 哈希链 |

## API 变更

### Products API

```typescript
// 响应类型 (无变化)
interface ProductResp {
  id: string;
  name: string;
  price: number;      // 依然是 f64 (元)，后端自动转换
  category: string;
  externalId: number;
  // ...
}

// 查询参数 (无变化)
interface FetchProductsParams {
  category?: string;
  search?: string;
  page?: number;
  limit?: number;
}

// 关键: 后端自动过滤 is_deleted=0 的产品
// 前端无需特殊处理
```

### Categories API

```typescript
// 响应类型
interface Category {
  name: string;
  sortOrder?: number;
  kitchenPrinterId?: number;
  isKitchenPrintEnabled: boolean;
  isLabelPrintEnabled: boolean;
}

// 后端自动过滤 is_deleted=0 的分类
```

### Orders API

```typescript
// 订单金额类型 (无变化，后端自动转换)
interface Order {
  orderId: number;
  subtotal: number;   // 元 (f64)
  total: number;      // 元 (f64)
  discount?: {
    type: 'PERCENTAGE' | 'FIXED_AMOUNT';
    value: number;
    amount: number;   // 元 (f64)
  };
  surcharge?: {
    type: 'PERCENTAGE' | 'FIXED_AMOUNT';
    amount: number;   // 元 (f64)
    total: number;    // 元 (f64)
  };
  items: OrderItem[];
}

// 订单明细项
interface OrderItem {
  id: string;         // product_id
  name: string;
  price: number;      // 元 (f64)
  quantity: number;
  originalPrice?: number;  // 元 (f64)
  selectedOptions?: OrderItemOption[];
}

interface OrderItemOption {
  attributeId: string;
  optionId: string;
  priceModifier: number;   // 元 (f64)
}
```

### Specifications API

```typescript
// 产品规格响应
interface ProductSpecificationResp {
  id: string;
  productId: string;
  name: string;
  receiptName?: string;
  price: number;      // 元 (f64)，后端自动转换
  externalId?: number;
  displayOrder: number;
  isDefault: boolean;
  isRoot: boolean;
  isActive: boolean;
  tags: SpecificationTagResp[];
}
```

## 价格处理

### 前端无需修改

前端代码保持不变，后端自动处理单位转换：

```typescript
// ✅ 前端代码无需修改
const product: ProductResp = {
  id: 'prod_1',
  name: 'Hamburguesa',
  price: 12.50,  // 12.50€ - 后端存储为 1250 分
};

// ✅ 直接使用即可
const total = items.reduce((sum, item) => sum + item.price * item.quantity, 0);
```

### 精度保证

```typescript
// 旧方案 (可能有精度问题)
const price1 = 0.1;
const price2 = 0.2;
const total = price1 + price2; // 0.30000000000000004

// ✅ 新方案: 后端使用分存储，完全精确
// 前端接收到的值已经是正确的 0.30
const price1 = 12.50;  // 从 API 获取
const price2 = 5.00;
const total = price1 + price2; // 17.50 (JavaScript 精度对 € 级别足够)
```

### 货币格式化

```typescript
// 使用现有工具函数
import { formatCurrency } from '@/utils/currency';

const price = 12.50;
const formatted = formatCurrency(price); // "12.50€"
```

## 软删除与过滤

### 后端自动处理

后端在查询时自动添加 `WHERE is_deleted = 0` 条件：

```typescript
// ✅ 前端无需任何修改
const products = await fetchProducts({ category: 'Bebidas' });
// 后端自动只返回未删除的产品
```

### 删除操作

```typescript
// ✅ 前端 API 调用不变
await deleteProduct('prod_1');
// 后端执行: UPDATE products SET is_deleted = 1 WHERE id = 'prod_1'

// ✅ 前端列表自动刷新后不显示已删除项
```

### 恢复删除 (如需要)

```typescript
// 如果前端需要支持恢复删除，调用新 API
await restoreProduct('prod_1');
// 后端执行: UPDATE products SET is_deleted = 0 WHERE id = 'prod_1'
```

## 类型映射

### Rust ↔ TypeScript 对照

| Rust 类型 (DB) | Rust 类型 (API) | TypeScript | 说明 |
|---------------|-----------------|------------|------|
| `i64` (price) | `f64` | `number` | 金额/价格 |
| `i64` (amount) | `f64` | `number` | 金额/数量 |
| `i32` (id) | `String` | `string` | ID (API层用String) |
| `bool` | `bool` | `boolean` | 布尔值 |
| `Option<T>` | `T\|null` | `T\|null\|undefined` | 可选值 |

### 完整类型定义

```typescript
// products.ts
export interface ProductResp {
  id: string;
  uuid: string;
  name: string;
  receiptName?: string;
  price: number;                    // € (f64)
  image: string;
  category: string;                 // category name
  externalId: number;
  taxRate: number;
  sortOrder?: number;
  kitchenPrinterId?: number;
  kitchenPrintName?: string;
  isKitchenPrintEnabled: number;    // -1: inherit, 0: off, 1: on
  isLabelPrintEnabled: number;      // -1: inherit, 0: off, 1: on
  hasMultiSpec?: boolean;
}

// categories.ts
export interface Category {
  name: string;
  sortOrder?: number;
  kitchenPrinterId?: number;
  isKitchenPrintEnabled: boolean;
  isLabelPrintEnabled: boolean;
}

// orders.ts
export interface Order {
  orderId: number;
  key: string;                      // table_id
  tableName?: string;
  receiptNumber?: string;
  status: string;
  startTime: number;
  endTime?: number;
  guestCount?: number;
  subtotal: number;                 // € (f64)
  total: number;                    // € (f64)
  discount?: {
    type: 'PERCENTAGE' | 'FIXED_AMOUNT';
    value: number;
    amount: number;                 // € (f64)
  };
  surcharge?: {
    type: 'PERCENTAGE' | 'FIXED_AMOUNT';
    amount: number;                 // € (f64)
    total: number;                  // € (f64)
  };
  zoneName?: string;
  items: OrderItem[];
  timeline?: TimelineEvent[];
}

export interface OrderItem {
  id: string;                       // product_id
  name: string;
  receiptName?: string;
  price: number;                    // € (f64)
  quantity: number;
  discountPercent?: number;
  surcharge?: number;
  guestId?: string;
  originalPrice?: number;           // € (f64)
  selectedOptions?: OrderItemOption[];
}

export interface OrderItemOption {
  attributeId: string;
  attributeName: string;
  optionId: string;
  optionName: string;
  receiptName?: string;
  priceModifier: number;            // € (f64)
}

// specifications.ts
export interface ProductSpecificationResp {
  id: string;
  productId: string;
  name: string;
  receiptName?: string;
  price: number;                    // € (f64)
  externalId?: number;
  displayOrder: number;
  isDefault: boolean;
  isRoot: boolean;
  isActive: boolean;
  tags: SpecificationTagResp[];
}
```

## 状态管理

### Cart Store

```typescript
// src/stores/cart.ts

// ✅ 无需修改 - 后端自动处理单位转换
interface CartItem {
  id: string;
  name: string;
  price: number;         // 12.50€ - 后端返回的就是元
  quantity: number;
  // ...
}

// 价格计算 (保持不变)
const calculateTotal = (items: CartItem[]): number => {
  return items.reduce((sum, item) => sum + item.price * item.quantity, 0);
};
```

### Order Store

```typescript
// src/stores/order.ts

// ✅ 无需修改
interface HeldOrder {
  key: string;
  items: CartItem[];
  subtotal: number;      // 元
  total: number;         // 元
  // ...
}
```

### Products Store

```typescript
// src/stores/products.ts

// ✅ fetchProducts 调用无需修改
const fetchProducts = async (params?: FetchProductsParams) => {
  const response = await invoke('fetch_products', { params });
  // response.products[].price 已经是元 (f64)
  return response;
};

// ✅ deleteProduct 调用无需修改
const deleteProduct = async (id: string) => {
  await invoke('delete_product', { id });
  // 后端执行软删除
};
```

## 常见问题

### Q: 前端代码需要修改吗？

**不需要**。前端 API 交互格式保持不变：
- 发送价格时: `12.50` (元)
- 接收价格时: `12.50` (元)
- 后端自动完成与数据库 `1250` (分) 的转换

### Q: 精度问题解决了吗？

**是的**。后端使用 `i64` (分) 存储，完全避免浮点精度问题：
- `0.1 + 0.2 = 0.3` ✓
- `12.50 + 5.00 = 17.50` ✓

### Q: 删除的产品还会显示吗？

**不会**。后端自动在查询时添加 `WHERE is_deleted = 0` 过滤。

### Q: 如何处理大额金额？

JavaScript 的 `number` 类型可以安全处理最多 `9,007,199,254,740,992` (约 9 千万亿) 的整数，对于分来说，相当于可以处理最多 `90,071,992,547,409.92€`，远超实际业务需求。

### Q: 需要修改货币格式化吗？

**不需要**。现有 `formatCurrency` 或类似函数继续使用：

```typescript
// 现有代码无需修改
const price = 12.50;
const formatted = `${price.toFixed(2)}€`; // "12.50€"
```

### Q: API 响应变了吗？

**基本不变**。API 响应格式与之前完全一致，只是后端存储方式改变。

### Q: 如何验证迁移是否成功？

1. 创建产品: `price = 12.50` → 数据库存储 `1250`
2. 读取产品: 返回 `price = 12.50` ✓
3. 删除产品: 产品从列表消失 ✓
4. 恢复产品: 产品重新出现 ✓

## 相关文件

| 文件 | 说明 |
|------|------|
| `src-tauri/migrations/20251215000000_init.sql` | 数据库迁移文件 |
| `src-tauri/migrations/MIGRATION_GUIDE.md` | 后端迁移指南 |
| `src/core/domain/types/` | TypeScript 类型定义 |
| `src/core/domain/validators.ts` | Zod 验证 Schema |
| `src/infrastructure/dataSource/validated.ts` | DataSource 验证包装器 |
| `src/infrastructure/apiValidator.ts` | 直接 API 调用验证工具 |

## Zod 校验层

### 已验证的数据源操作

所有通过 `DataSourceFactory.create()` 创建的数据源都会自动进行 Zod 校验：

```typescript
// 验证的数据源操作
const products = await dataSource.fetchProducts(params);  // ✅ 已验证
const categories = await dataSource.fetchCategories();    // ✅ 已验证
const tables = await dataSource.fetchTables(params);      // ✅ 已验证
const printers = await dataSource.fetchKitchenPrinters(); // ✅ 已验证
```

### 验证的 API 模块

```typescript
// Price Adjustments API - 已添加 Zod 验证
import { fetchAdjustmentRules, getAdjustmentRule } from '@/services/api/price_adjustments';

const rules = await fetchAdjustmentRules();  // ✅ 自动验证响应格式
```

### 错误处理

验证失败时会抛出 `ValidationError`：

```typescript
try {
  const product = await dataSource.getProduct(id);
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('数据验证失败:', error.message);
    console.error('上下文:', error.context);
  }
}
```

## 迁移验证清单

- [x] 价格处理：后端自动完成分↔元转换，前端无需修改
- [x] 软删除：后端自动过滤 `is_deleted=0` 的数据
- [x] 类型定义：TypeScript 类型与 Rust 后端保持一致
- [x] Zod 验证：核心 API 响应已添加运行时校验
- [x] DataSource：统一的数据访问层带验证包装
- [x] 类型检查：`npx tsc --noEmit` 通过
