# 会员 & Marketing Group 设计方案

> 日期: 2026-02-11
> 状态: Draft
> 范围: shared, edge-server, red_coral, crab-auth

## 概述

为 Crab POS 系统引入**会员管理**和 **Marketing Group（营销组）** 功能。Marketing Group 是会员等级的实现载体，每个 MG 定义了该等级可享受的营销权益（折扣规则 + 集章活动）。

**此功能为 Pro plan 专属**，需要首次启用 feature gating 机制。

## 核心概念

```
Member ──(N:1)──→ MarketingGroup ──(1:N)──→ MgDiscountRule
                                  ──(1:N)──→ StampActivity

MG = 会员等级 = 营销权益包
```

- **Member**: 会员实体，绑定一个 MG
- **MarketingGroup**: 等级 + 权益容器（无需独立 tier 概念）
- **MgDiscountRule**: MG 专属折扣规则，**独立于 PriceRule**
- **StampActivity**: 集章活动，按符合条件的商品数量盖章

### 与 PriceRule 的关系

| 维度 | PriceRule | Marketing Group |
|------|-----------|-----------------|
| 定位 | 通用定价策略 | 会员营销权益 |
| 示例 | Happy Hour、区域附加费 | 金卡 9 折、集章买10送1 |
| 叠加 | 内部叠加（排他/可叠加/不可叠加） | MG 规则间全部叠加 |
| 跨系统 | 与 MG 折扣**叠乘**（资本家模式） | 与 PriceRule **叠乘** |
| 时间约束 | 支持 | 不支持（MG 级别始终生效） |
| 区域约束 | 支持 | 不支持 |

## 数据模型

### Member（会员）

```sql
CREATE TABLE members (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    phone       TEXT,                    -- 手机号（可搜索）
    card_number TEXT,                    -- 会员卡号/二维码标识
    marketing_group_id INTEGER NOT NULL, -- 绑定的 MG
    birthday    TEXT,                    -- MM-DD 格式（预留生日特权）
    points_balance INTEGER NOT NULL DEFAULT 0, -- 积分余额（预留）
    notes       TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL,        -- Unix 毫秒
    updated_at  INTEGER NOT NULL,

    FOREIGN KEY (marketing_group_id) REFERENCES marketing_groups(id)
);

CREATE INDEX idx_members_phone ON members(phone);
CREATE INDEX idx_members_card_number ON members(card_number);
CREATE INDEX idx_members_marketing_group ON members(marketing_group_id);
```

**会员识别方式**: 手机号 / 会员卡二维码 / 姓名编号搜索

### MarketingGroup（营销组 = 会员等级）

```sql
CREATE TABLE marketing_groups (
    id           INTEGER PRIMARY KEY,
    name         TEXT NOT NULL UNIQUE,    -- 内部标识
    display_name TEXT NOT NULL,           -- 前端展示 "金卡会员"
    description  TEXT,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    -- 预留积分配置
    points_earn_rate  REAL,              -- 积分倍率 (null=不启用)
    points_per_unit   REAL,              -- 每消费多少元积1分
    is_active    INTEGER NOT NULL DEFAULT 1,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);
```

### MgDiscountRule（MG 折扣规则）

```sql
CREATE TABLE mg_discount_rules (
    id                  INTEGER PRIMARY KEY,
    marketing_group_id  INTEGER NOT NULL,
    name                TEXT NOT NULL,
    display_name        TEXT NOT NULL,
    receipt_name        TEXT NOT NULL,       -- 收据打印名

    product_scope       TEXT NOT NULL,       -- GLOBAL / CATEGORY / PRODUCT
    target_id           INTEGER,             -- 分类/产品 ID

    adjustment_type     TEXT NOT NULL,       -- PERCENTAGE / FIXED_AMOUNT
    adjustment_value    REAL NOT NULL,       -- 10 = 10% 或 €10

    is_active           INTEGER NOT NULL DEFAULT 1,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL,

    FOREIGN KEY (marketing_group_id) REFERENCES marketing_groups(id)
);

CREATE INDEX idx_mg_discount_rules_group ON mg_discount_rules(marketing_group_id);
```

**与 PriceRule 的区别**: 无时间约束、无区域范围、无叠加控制（MG 内规则全部叠加）。

### StampActivity（集章活动）

```sql
CREATE TABLE stamp_activities (
    id                    INTEGER PRIMARY KEY,
    marketing_group_id    INTEGER NOT NULL,
    name                  TEXT NOT NULL,
    display_name          TEXT NOT NULL,

    -- 集章条件（通过 stamp_targets 关联表指定哪些分类/产品可盖章）
    stamps_required       INTEGER NOT NULL,  -- 集满几个章 (如 10)

    -- 奖励配置（通过 stamp_reward_targets 关联表指定可赠送的分类/产品）
    reward_quantity       INTEGER NOT NULL DEFAULT 1, -- 赠送数量
    reward_strategy       TEXT NOT NULL DEFAULT 'ECONOMIZADOR', -- ECONOMIZADOR / GENEROSO / DESIGNATED
    designated_product_id INTEGER,       -- Designated 策略时指定的商品 ID

    -- 行为
    is_cyclic             INTEGER NOT NULL DEFAULT 1, -- 集满后是否重新开始
    is_active             INTEGER NOT NULL DEFAULT 1,
    created_at            INTEGER NOT NULL,
    updated_at            INTEGER NOT NULL,

    FOREIGN KEY (marketing_group_id) REFERENCES marketing_groups(id)
);

CREATE INDEX idx_stamp_activities_group ON stamp_activities(marketing_group_id);
```

### StampTarget（集章条件目标 — 多对多）

通过此表指定哪些分类/产品可以盖章（必须至少有一条记录）。

```sql
CREATE TABLE stamp_targets (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL,
    target_type       TEXT NOT NULL,     -- CATEGORY / PRODUCT
    target_id         INTEGER NOT NULL,  -- 分类 ID 或产品 ID

    FOREIGN KEY (stamp_activity_id) REFERENCES stamp_activities(id) ON DELETE CASCADE
);

CREATE INDEX idx_stamp_targets_activity ON stamp_targets(stamp_activity_id);
```

**示例**:
- 买奶茶类或果茶类都盖章：2 条记录，`target_type = CATEGORY`
- 买 5 种指定商品才盖章：5 条记录，`target_type = PRODUCT`

### StampRewardTarget（集章奖励目标 — 多对多）

Economizador/Generoso 策略时，通过此表指定可赠送的目标（必须至少有一条记录）。
Designated 策略时忽略此表，直接用 `designated_product_id`。

```sql
CREATE TABLE stamp_reward_targets (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL,
    target_type       TEXT NOT NULL,     -- CATEGORY / PRODUCT
    target_id         INTEGER NOT NULL,  -- 分类 ID 或产品 ID

    FOREIGN KEY (stamp_activity_id) REFERENCES stamp_activities(id) ON DELETE CASCADE
);

CREATE INDEX idx_stamp_reward_targets_activity ON stamp_reward_targets(stamp_activity_id);
```

**示例**:
- 赠送 5 种奶茶中的任意一种：5 条记录，`target_type = PRODUCT`
- 赠送饮品类或甜品类中的任意一个：2 条记录，`target_type = CATEGORY`

Economizador/Generoso 策略从订单中匹配这些目标，选最便宜/最贵的 comp。

### MemberStampProgress（会员集章进度）

```sql
CREATE TABLE member_stamp_progress (
    id                INTEGER PRIMARY KEY,
    member_id         INTEGER NOT NULL,
    stamp_activity_id INTEGER NOT NULL,
    current_stamps    INTEGER NOT NULL DEFAULT 0,
    completed_cycles  INTEGER NOT NULL DEFAULT 0, -- cyclic 模式下已完成轮次
    last_stamp_at     INTEGER,                     -- 最后盖章时间
    updated_at        INTEGER NOT NULL,

    FOREIGN KEY (member_id) REFERENCES members(id),
    FOREIGN KEY (stamp_activity_id) REFERENCES stamp_activities(id),
    UNIQUE(member_id, stamp_activity_id)
);
```

### 赠送策略（RewardStrategy）

```rust
pub enum RewardStrategy {
    /// 免除订单内符合条件的**最低价**商品（默认，省钱型）
    Economizador,
    /// 免除订单内符合条件的**最高价**商品（大方型，适合新店开张搞噱头）
    Generoso,
    /// 指定固定商品赠送（如「永远送一杯美式」），不依赖订单已有商品
    Designated,
}
```

| 策略 | 说明 | 行为 |
|------|------|------|
| **Economizador** | 最低价（默认） | 从订单中匹配 `stamp_reward_targets` 的商品里选**最便宜**的 comp |
| **Generoso** | 最高价 | 从订单中匹配 `stamp_reward_targets` 的商品里选**最贵**的 comp |
| **Designated** | 指定商品 | 直接添加 `designated_product_id` 指定的商品作为 comp，不依赖订单已有商品 |

- Economizador / Generoso：需要订单中有符合目标的商品，没有则提示
- Designated：无需订单中已有该商品，系统自动添加
- 赠送目标支持**多个**（通过 `stamp_reward_targets` 关联表）

**示例**:
- 集章范围：`Category = 奶茶`（买奶茶盖章）
- 赠送目标：5 种奶茶（珍珠、抹茶、红茶...），Economizador 从中选最便宜的
- 或指定赠送：`Designated = 美式咖啡`（固定送美式，忽略 reward_targets）

### 集章触发规则

- 按**符合条件的商品数量**盖章（买 2 杯 = 2 个章）
- 只有匹配 `stamp_targets` 表中的分类/产品才算盖章（必须配置目标）
- 只统计非赠送（non-comp）商品

## 订单流程集成

### 会员绑定流程

```
收银员搜索会员（手机号/卡号/姓名）
    │
    ▼
选中会员 → 发送 LinkMember 命令
    │
    ▼
OrderEvent::MemberLinked { member_id, member_name, mg_id, mg_name }
    │
    ▼
加载 MG 折扣规则 → 重新计算所有商品价格
加载 MG 集章活动 → 统计当前订单符合条件的商品数量（预览）
```

### 价格计算叠加顺序

```
1. base_price = 商品原价 + 选项加价
2. manual_discount → 手动折扣
3. price_rule_discount → PriceRule 折扣（Happy Hour 等）
4. mg_discount → MG 折扣（会员专属）           ← 新增
5. surcharges → PriceRule 附加费
6. final_price = 叠加后的最终价格
```

- MG 折扣在 PriceRule 折扣之后应用
- MG 折扣与 PriceRule 折扣**叠乘**（资本家模式）
- MG 折扣规则之间也叠乘

### 集章追踪流程

```
订单完成（Completed）时
    │
    ▼
遍历订单中所有非赠送商品
    │
    ▼
匹配会员 MG 的所有活跃集章活动
    │
    ▼
按商品数量累加 MemberStampProgress.current_stamps
    │
    ▼
若 current_stamps >= stamps_required
    ├── 标记可兑换（不自动兑换）
    └── 通知前端展示「可兑换」状态
```

**关键决策**: 集章在**订单完成时**才写入（不是下单时），避免取消订单需要回退章数。

### 集章兑换流程

```
收银员点击「兑换集章奖励」
    │
    ▼
选择要兑换的集章活动
    │
    ▼
系统按 reward_strategy 在订单中查找符合 reward_product_scope 的商品:
    ├── Economizador → 选最便宜的
    └── Generoso     → 选最贵的
    │
    ▼
将选中商品 comp（赠送），生成事件:
OrderEvent::StampRedeemed {
    stamp_activity_id,
    stamp_activity_name,
    reward_item_id,     // 被赠送的商品
    reward_strategy,    // 使用的策略
}
    │
    ▼
MemberStampProgress 更新:
    ├── is_cyclic = true  → current_stamps = 0, completed_cycles += 1
    └── is_cyclic = false → 标记活动已完成（不再累计）
```

### OrderSnapshot 扩展

```rust
pub struct OrderSnapshot {
    // ...现有字段...

    // 会员信息
    pub member_id: Option<i64>,
    pub member_name: Option<String>,
    pub marketing_group_id: Option<i64>,
    pub marketing_group_name: Option<String>,

    // MG 折扣追踪（类似 applied_rules）
    pub applied_mg_rules: Vec<AppliedMgRule>,
}

pub struct AppliedMgRule {
    pub rule_id: i64,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub product_scope: ProductScope,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,      // 原始值 (10%)
    pub calculated_amount: f64,     // 计算后金额 (€10)
    pub skipped: bool,              // 收银员可手动跳过
}
```

### 事件溯源扩展

```rust
enum OrderEvent {
    // ...现有事件...

    /// 会员绑定到订单
    MemberLinked {
        member_id: i64,
        member_name: String,
        marketing_group_id: i64,
        marketing_group_name: String,
    },

    /// 会员解绑
    MemberUnlinked,

    /// 集章奖励兑换
    StampRedeemed {
        stamp_activity_id: i64,
        stamp_activity_name: String,
        reward_item_id: i64,
    },
}
```

## 模块架构

### Rust 模块结构

```
shared/src/models/
├── member.rs              ← Member
├── marketing_group.rs     ← MarketingGroup
├── mg_discount_rule.rs    ← MgDiscountRule
└── stamp.rs               ← StampActivity, MemberStampProgress

shared/src/order/
└── applied_mg_rule.rs     ← AppliedMgRule

edge-server/src/
├── db/repository/
│   ├── member.rs          ← Member CRUD + 搜索
│   ├── marketing_group.rs ← MG + 关联规则/活动 CRUD
│   └── stamp.rs           ← 集章进度读写
│
├── marketing/             ← 新模块：营销引擎（独立于 pricing/）
│   ├── mod.rs
│   ├── mg_calculator.rs   ← MG 折扣计算
│   └── stamp_tracker.rs   ← 集章追踪 & 兑换
│
├── api/
│   ├── members/           ← 会员管理 API (CRUD + 搜索)
│   └── marketing_groups/  ← MG 管理 API (含规则 + 活动)
│
└── orders/actions/
    ├── link_member.rs     ← 绑定会员命令
    ├── unlink_member.rs   ← 解绑会员命令
    └── redeem_stamp.rs    ← 集章兑换命令
```

### MG 规则缓存

复用现有 PriceRule 的缓存模式：
- 会员绑定到订单时，缓存该 MG 的折扣规则（内存 + redb）
- 订单终结时清除缓存
- 服务重启时从 redb 恢复

## Feature Gate（功能门控）

### 方案

利用 `SubscriptionInfo.features: Vec<String>` 字段（已存在但未使用）。

**crab-auth 侧**:
- Pro plan 签发的 SubscriptionInfo 包含 `features: ["marketing"]`
- Basic plan 不包含 `"marketing"` feature

**edge-server 侧**:

```rust
// 新增 feature gate 检查函数
pub fn require_feature(feature: &str) -> impl Fn(&SubscriptionInfo) -> bool {
    move |sub| sub.features.contains(&feature.to_string())
}

// 在会员/MG 相关 API 路由上应用
Router::new()
    .route("/api/members", get(list_members).post(create_member))
    .route("/api/marketing-groups", get(list_groups).post(create_group))
    .layer(middleware::from_fn(require_feature_middleware("marketing")))
```

**前端侧**:
- `SubscriptionInfo.features` 已通过 AppState 传递到前端
- 前端检查 `features.includes("marketing")` 决定是否显示会员相关 UI
- 不满足时隐藏入口或显示「Pro plan 专属」提示

### Feature 定义

| Feature Key | Plan | 说明 |
|-------------|------|------|
| `marketing` | Pro+ | 会员管理、Marketing Group、折扣规则、集章活动 |

## 权限扩展

### 新增 RBAC 权限

```
members:manage           // 会员管理 (CRUD)
marketing_groups:manage  // 营销组管理 (CRUD + 规则 + 活动)
orders:link_member       // 订单绑定/解绑会员
orders:redeem_stamp      // 集章兑换
```

## API 设计

### 会员管理

```
GET    /api/members                  ← 列表 + 搜索 (?q=手机号/姓名/卡号)
POST   /api/members                  ← 创建会员
GET    /api/members/:id              ← 会员详情（含集章进度）
PUT    /api/members/:id              ← 更新会员信息
DELETE /api/members/:id              ← 停用会员 (soft delete)
```

### Marketing Group 管理

```
GET    /api/marketing-groups                        ← 列表
POST   /api/marketing-groups                        ← 创建 MG
GET    /api/marketing-groups/:id                    ← 详情（含规则 + 活动）
PUT    /api/marketing-groups/:id                    ← 更新 MG
DELETE /api/marketing-groups/:id                    ← 停用 MG

POST   /api/marketing-groups/:id/discount-rules     ← 添加折扣规则
PUT    /api/marketing-groups/:id/discount-rules/:rid ← 更新折扣规则
DELETE /api/marketing-groups/:id/discount-rules/:rid ← 删除折扣规则

POST   /api/marketing-groups/:id/stamp-activities    ← 添加集章活动
PUT    /api/marketing-groups/:id/stamp-activities/:sid ← 更新集章活动
DELETE /api/marketing-groups/:id/stamp-activities/:sid ← 删除集章活动
```

### 订单命令（通过 Tauri 命令）

```
link_member(order_id, member_id)       ← 绑定会员到订单
unlink_member(order_id)                ← 解绑会员
redeem_stamp(order_id, stamp_activity_id, product_id) ← 兑换集章奖励
```

## 前端页面

### 新增页面/组件

1. **会员管理页面** — 会员 CRUD、搜索、查看集章进度
2. **Marketing Group 管理页面** — MG CRUD、折扣规则配置、集章活动配置
3. **订单侧边栏：会员面板** — 搜索绑定会员、显示当前权益、集章进度、兑换按钮
4. **Pro plan 升级提示** — 非 Pro 用户尝试访问时显示

## 实现阶段

### Phase 1: 基础设施
- 数据库 schema (migrations)
- Rust 模型 (shared/)
- Repository 层 (edge-server/db/)
- Feature gate 机制

### Phase 2: 营销引擎
- MG 折扣计算器 (marketing/mg_calculator.rs)
- 集章追踪器 (marketing/stamp_tracker.rs)
- 价格计算叠加集成

### Phase 3: API & 命令
- 会员管理 API
- MG 管理 API
- 订单命令 (link/unlink/redeem)

### Phase 4: 前端
- 会员管理页面
- MG 管理页面
- 订单中会员面板
- Feature gate UI

## 不做的事情（第一阶段）

- ❌ 积分兑换逻辑（字段预留，逻辑不实现）
- ❌ 生日特权自动触发（字段预留）
- ❌ 自动升降级（手动分配等级）
- ❌ 储值/充值余额
- ❌ 优惠券系统
- ❌ 消费历史分析
