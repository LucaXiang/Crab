# 厨房打印 & 标签打印功能设计

## 概述

实现下单自动打印功能：
- **厨房打印**：按打印目的地分组，发送厨房单到各厨房/出菜口
- **标签打印**：每个菜品打印标签（如奶茶贴纸），支持多打印机

## 核心概念

### 打印目的地 (PrintDestination)

已有模型，代表一个打印位置（厨房、出菜口、吧台等），用户可配置任意数量。

### 打印路由

**回退链（优先级从高到低）：**
```
商品配置 → 分类配置 → 系统默认 → 未配置=功能禁用
```

**系统默认配置：**
- `default_kitchen_printer`: 默认厨房打印机（最终回退）
- `default_label_printer`: 默认标签打印机（最终回退）
- 未配置系统默认 = 该功能未开启

**厨房打印：**
- 商品 `kitchen_print_destinations` > 分类 `kitchen_print_destinations` > 系统默认
- 全都没配置 = 不打印

**标签打印：**
- 需要 `is_label_print_enabled = true`
- 商品 `label_print_destinations` > 分类 `label_print_destinations` > 系统默认
- 全都没配置 = 不打印

**性能优化：**
- ItemsAdded 时先检查系统默认是否配置
- 系统默认未配置 = 功能未开启，直接跳过，零开销

### 厨房票据内容

- 桌号、下单时间（MM-DD HH:mm:ss）
- 按分类分组，组内按 `external_id` 排序
- 商品编号（root spec 的 `external_id`）
- 厨房打印名称（`kitchen_print_name` ?? `name`）
- 数量、规格
- 属性/做法（根据 `Attribute.show_on_kitchen_print` 过滤）
- 备注

```
┌────────────────────────────┐
│  100桌    01-22 14:32:15     │
├────────────────────────────┤
│ 【热菜】                    │
│  #001 宫保鸡丁 (大) x2      │
│       - 微辣               │
│       * 不要花生            │
│  #003 红烧肉 x1             │
├────────────────────────────┤
│ 【凉菜】                    │
│  #015 凉拌黄瓜 x1           │
│       * 少放蒜              │
├────────────────────────────┤
│ 【主食】                    │
│  #020 蛋炒饭 x2             │
│       - 加蛋               │
└────────────────────────────┘
```

## 数据模型

### Category 扩展

```rust
// shared/src/models/category.rs
pub struct Category {
    // ... 现有字段

    /// 默认打印目的地（商品可覆盖）
    #[serde(default)]
    pub kitchen_print_destinations: Vec<String>,
}
```

### KitchenOrder（点单记录）

存储在 redb，以 ItemsAdded 事件为单位：

```rust
// edge-server/src/kitchen/types.rs

/// 一次点单的厨房记录（对应一个 ItemsAdded 事件）
pub struct KitchenOrder {
    pub id: String,                      // = event_id
    pub order_id: String,
    pub table_name: Option<String>,
    pub created_at: i64,                 // 时间戳
    pub items: Vec<KitchenOrderItem>,
    pub print_count: u32,                // 打印次数（0=未打印，>1=补发过）
}

/// 菜品打印上下文（完整 JSON，模板自取所需字段）
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

pub struct KitchenOrderItem {
    pub context: PrintItemContext,       // 完整上下文
}

// 打印排序规则：
// 1. 按 category_id 分组
// 2. 组内按 external_id 升序

/// 标签打印记录（单品级别）
pub struct LabelPrintRecord {
    pub id: String,                      // UUID
    pub order_id: String,
    pub kitchen_order_id: String,        // 关联的 KitchenOrder
    pub table_name: Option<String>,
    pub created_at: i64,
    pub context: PrintItemContext,       // 完整上下文
    pub print_count: u32,                // 打印次数
}
```

### 打印配置缓存

```rust
// edge-server/src/kitchen/cache.rs

/// 商品打印配置（内存缓存）
pub struct ProductPrintConfig {
    pub product_id: String,
    pub product_name: String,
    pub kitchen_name: String,                    // kitchen_print_name ?? name
    pub kitchen_print_destinations: Vec<String>,         // 厨房
    pub label_print_destinations: Vec<String>,   // 标签
    pub is_label_print_enabled: bool,
    pub root_spec_external_id: Option<i64>,
    pub category_id: String,
}

/// 分类打印配置（内存缓存）
pub struct CategoryPrintConfig {
    pub category_id: String,
    pub category_name: String,
    pub kitchen_print_destinations: Vec<String>,         // 厨房
    pub label_print_destinations: Vec<String>,   // 标签
    pub is_label_print_enabled: bool,
}
```

## 架构

```
┌─────────────────────────────────────────────────────────────┐
│                     KitchenPrintService                      │
├─────────────────────────────────────────────────────────────┤
│  config_cache: HashMap<ProductId, ProductPrintConfig>        │
│  category_cache: HashMap<CategoryId, CategoryPrintConfig>    │
│  storage: redb (kitchen_orders table)                        │
│  printer_pool: 网络打印机连接池                               │
│  enabled: bool (总开关)                                       │
├─────────────────────────────────────────────────────────────┤
│  事件监听:                                                    │
│    ItemsAdded → 生成 KitchenOrder → 按目的地拆分打印          │
│    OrderCompleted/Voided → 清理该订单的 KitchenOrder          │
├─────────────────────────────────────────────────────────────┤
│  API:                                                        │
│    GET /kitchen-orders?order_id=xxx → 全量返回该订单记录      │
│    GET /kitchen-orders?page=1&limit=20 → 分页获取全部         │
│    POST /kitchen-orders/{id}/reprint → 补发打印               │
│    POST /kitchen-orders/refresh-cache → 刷新配置缓存          │
└─────────────────────────────────────────────────────────────┘
```

## 流程

### 1. 下单自动打印

```
AddItems Command
    ↓
OrdersManager 生成 ItemsAdded 事件
    ↓
广播事件
    ↓
KitchenPrintService 监听到 ItemsAdded
    ↓
快速检查：系统默认打印机是否配置？
  - 厨房/标签都未配置 → 直接返回（零开销）
    ↓
根据 product_id 从缓存查询打印配置
  - 商品有配置 → 使用商品的 kitchen_print_destinations
  - 商品无配置 → 回退到分类的 kitchen_print_destinations
  - 都没有 → 该菜品不打印
    ↓
创建 KitchenOrder，存入 redb
    ↓
按目的地分组，生成多张厨房票据
    ↓
发送打印指令到各打印机
    ↓
更新 print_count = 1
```

### 2. 补发打印

```
前端选择某条 KitchenOrder 点击补发
    ↓
KitchenPrintService.reprint(id)
    ↓
从 redb 读取 KitchenOrder
    ↓
按目的地重新分组打印
    ↓
print_count++
```

### 3. 标签打印流程

```
ItemsAdded 事件
    ↓
检查菜品 is_label_print_enabled
    ↓
为每个启用的菜品创建 LabelPrintRecord
  - quantity=3 → 创建 3 条记录（index: 1/3, 2/3, 3/3）
    ↓
按目的地分组，发送打印任务
    ↓
存入 redb
```

### 4. 标签补打

- **单品级别补打**：选择某个 LabelPrintRecord 重打
- 补打时 `print_count++`

### 5. 数据清理

- **保留 3 天**：KitchenOrder + LabelPrintRecord 不随订单关闭删除
- **启动时**：清理超过 3 天（72 小时）的记录
- **定时清理**：可选，每小时检查一次

## 打印机配置

### PrintDestination 扩展

```rust
pub struct EmbeddedPrinter {
    pub printer_type: String,      // "network" | "driver"
    pub printer_format: String,    // "escpos" | "label"  ← 新增
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub driver_name: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}
```

### 打印机类型

| printer_format | 用途 | 纸张 |
|----------------|------|------|
| `escpos` | 厨房单、收银小票 | 80mm 热敏纸 |
| `label` | 标签（奶茶贴纸） | 自定义尺寸 |

### 打印架构

**edge-server 直接和打印机通信**（不通过 Client 中转）：

```
edge-server ──TCP/IP──→ 网络打印机（厨房、标签）
            ──驱动───→ 本地打印机（Server模式同机）

Client ──驱动──→ 本地收银打印机（独立控制）
```

- **网络打印机**：edge-server 直接 TCP/IP 发送指令
- **Server 模式**：edge-server 本机可能安装打印机，直接调用驱动
- **Client 本地**：收银小票、开钱箱由 Client 自己控制，与 edge-server 无关

### 打印机分类

| 类型 | 控制方 | 用途 | 配置位置 |
|------|--------|------|----------|
| **网络打印机** | edge-server (TCP/IP) | 厨房、标签 | PrintDestination |
| **服务端本地打印机** | edge-server (驱动) | 厨房、标签 | PrintDestination |
| **Client 本地打印机** | Client (驱动) | 收银小票、开钱箱 | Client 本地存储 |

### 前端打印机配置

前端需区分：
- **远程打印机**：配置在 edge-server（厨房/标签），同步到服务端
- **本地打印机**：Client 本地配置，存本地存储
  - 收银小票打印机
  - 开钱箱指令

## 标签内容示例

```
┌──────────────────┐
│  #001 拿铁        │
│  大杯 / 少糖少冰   │
│  + 珍珠           │
│  * 不要吸管        │
│ ──────────────── │
│  100桌    2/5    │
└──────────────────┘
      ↑ 第2杯/共5杯
```

标签数据通过 JSON 注入模板渲染（现有 LabelTemplate 机制）。

## 前端界面

### 数据获取策略

- **订单级别**：全量拉取（数量少，不分页）
- **全局列表**：分页获取（按时间倒序）
- **不缓存**：每次进入页面重新拉取

### 厨房小票列表页

显示所有活跃订单的点单记录（分页）：

```
┌──────────────────────────────────────────────────┐
│  厨房小票                              [刷新]    │
├──────────────────────────────────────────────────┤
│  🍽 100桌  14:32  (已打印x1)           [补发]   │
│     #001 宫保鸡丁 (大份) x2                      │
│          - 微辣、少油                            │
│     #015 冰淇淋 x1                               │
├──────────────────────────────────────────────────┤
│  🍽 88桌   14:28  (已打印x1)           [补发]   │
│     #003 红烧肉 x1                               │
│     #007 青菜 x2                                 │
└──────────────────────────────────────────────────┘
```

### 设置页

- 厨房打印总开关
- 打印目的地管理（已有）
- 分类默认打印目的地配置
- 商品打印目的地配置（已有）

## 缓存更新

### 触发时机

- 服务启动时：全量加载
- 商品 CRUD：更新对应商品缓存
- 分类 CRUD：更新对应分类缓存

### 缓存结构

```rust
struct PrintConfigCache {
    products: HashMap<String, ProductPrintConfig>,
    categories: HashMap<String, CategoryPrintConfig>,

    // 系统默认（最终回退，未配置=功能禁用）
    default_kitchen_printer: Option<String>,
    default_label_printer: Option<String>,
}

impl PrintConfigCache {
    /// 厨房打印功能是否启用
    fn is_kitchen_print_enabled(&self) -> bool {
        self.default_kitchen_printer.is_some()
    }

    /// 标签打印功能是否启用
    fn is_label_print_enabled(&self) -> bool {
        self.default_label_printer.is_some()
    }

    /// 获取厨房打印目的地（商品 > 分类 > 系统默认）
    fn get_kitchen_destinations(&self, product_id: &str) -> Vec<String> {
        if let Some(product) = self.products.get(product_id) {
            if !product.kitchen_print_destinations.is_empty() {
                return product.kitchen_print_destinations.clone();
            }
            if let Some(category) = self.categories.get(&product.category_id) {
                if !category.kitchen_print_destinations.is_empty() {
                    return category.kitchen_print_destinations.clone();
                }
            }
        }
        // 最终回退到系统默认
        self.default_kitchen_printer.iter().cloned().collect()
    }

    /// 获取标签打印目的地（商品 > 分类 > 系统默认）
    fn get_label_destinations(&self, product_id: &str) -> Vec<String> {
        if let Some(product) = self.products.get(product_id) {
            // 先检查是否启用标签打印
            let enabled = product.is_label_print_enabled
                || self.categories.get(&product.category_id)
                    .map(|c| c.is_label_print_enabled)
                    .unwrap_or(false);

            if !enabled {
                return vec![];
            }

            if !product.label_print_destinations.is_empty() {
                return product.label_print_destinations.clone();
            }
            if let Some(category) = self.categories.get(&product.category_id) {
                if !category.label_print_destinations.is_empty() {
                    return category.label_print_destinations.clone();
                }
            }
        }
        // 最终回退到系统默认
        self.default_label_printer.iter().cloned().collect()
    }
}
```

## redb 表设计

```rust
// 新增表
const KITCHEN_ORDERS_TABLE: TableDefinition<&str, &[u8]>
    = TableDefinition::new("kitchen_orders");
// key = kitchen_order_id, value = JSON-serialized KitchenOrder

// 索引表：按 order_id 查询
const KITCHEN_ORDERS_BY_ORDER_TABLE: TableDefinition<(&str, &str), ()>
    = TableDefinition::new("kitchen_orders_by_order");
// key = (order_id, kitchen_order_id), value = ()
```

## 注意事项

1. **打印失败无反馈**：网络打印机通常没有回执，只能记录"已发送"
2. **补发可能重复**：服务员口头告知厨房忽略重复单
3. **Client 模式**：厨房打印由 edge-server 控制，Client 只做显示和触发补发
4. **本地打印**：收银小票、标签打印由 Client 本地处理，不在此设计范围

## 实现步骤

### 基础设施
1. [ ] `EmbeddedPrinter` 添加 `printer_format` 字段（escpos/label）
2. [ ] Category 模型添加 `kitchen_print_destinations` 字段（厨房）
3. [ ] Category 模型添加 `label_print_destinations` 字段（标签）
4. [ ] Product 模型添加 `label_print_destinations` 字段（标签）
5. [ ] 创建 `edge-server/src/printing/` 模块

### 厨房打印
4. [ ] 实现 `PrintConfigCache`（商品/分类打印配置缓存）
5. [ ] 实现 `KitchenOrder` redb 存储
6. [ ] 实现 `KitchenPrintService`
7. [ ] 集成到 `OrdersManager` 事件监听
8. [ ] 实现 ESC/POS 厨房单渲染

### 标签打印
9. [ ] 实现 `LabelPrintRecord` redb 存储
10. [ ] 实现标签打印触发（检查 `is_label_print_enabled`）
11. [ ] 生成标签 JSON 数据

### API
12. [ ] `GET /kitchen-orders` - 厨房单列表
13. [ ] `POST /kitchen-orders/{id}/reprint` - 厨房单补发
14. [ ] `GET /label-records` - 标签记录列表
15. [ ] `POST /label-records/{id}/reprint` - 标签补发

### 前端
16. [ ] 设置页：打印机类型选择（escpos/label）
17. [ ] 设置页：分类默认打印目的地配置
18. [ ] 厨房小票列表页 + 补发
19. [ ] 标签记录列表页 + 补发（单品级别）
