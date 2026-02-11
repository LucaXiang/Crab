# 会员 & Marketing Group 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 Crab POS 系统实现会员管理 + Marketing Group 营销系统（折扣规则 + 集章活动），Pro plan 专属功能。

**Architecture:** 在 shared/ 定义模型和事件类型，edge-server/ 实现 repository + marketing 引擎 + API + 订单集成。Marketing Group 独立于 PriceRule，两者在订单结算时叠乘。集章进度持久化到 SQLite，兑换复用 comp 机制。

**Tech Stack:** Rust (shared + edge-server), SQLite (sqlx), Axum (API), serde, rust_decimal

**Design Doc:** `docs/plans/2026-02-11-membership-marketing-group-design.md`

---

## Phase 1: Database Schema

### Task 1: Create migration file

**Files:**
- Create: `edge-server/migrations/0002_marketing_groups.sql`

**Step 1: Create migration**

```bash
sqlx migrate add -r -s marketing_groups --source edge-server/migrations
```

这会生成 `0002_marketing_groups.up.sql` 和 `0002_marketing_groups.down.sql`。

**Step 2: Write UP migration**

写入 `0002_marketing_groups.up.sql`:

```sql
-- ── Marketing Groups (营销组 = 会员等级) ─────────────────────
CREATE TABLE marketing_group (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL UNIQUE,
    display_name TEXT    NOT NULL,
    description  TEXT,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    points_earn_rate  REAL,
    points_per_unit   REAL,
    is_active    INTEGER NOT NULL DEFAULT 1,
    created_at   INTEGER NOT NULL DEFAULT 0,
    updated_at   INTEGER NOT NULL DEFAULT 0
);

-- ── Members (会员) ──────────────────────────────────────────
CREATE TABLE member (
    id                 INTEGER PRIMARY KEY,
    name               TEXT    NOT NULL,
    phone              TEXT,
    card_number        TEXT,
    marketing_group_id INTEGER NOT NULL REFERENCES marketing_group(id),
    birthday           TEXT,
    points_balance     INTEGER NOT NULL DEFAULT 0,
    notes              TEXT,
    is_active          INTEGER NOT NULL DEFAULT 1,
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_member_phone ON member(phone);
CREATE INDEX idx_member_card_number ON member(card_number);
CREATE INDEX idx_member_marketing_group ON member(marketing_group_id);

-- ── MG Discount Rules (MG 折扣规则) ────────────────────────
CREATE TABLE mg_discount_rule (
    id                 INTEGER PRIMARY KEY,
    marketing_group_id INTEGER NOT NULL REFERENCES marketing_group(id),
    name               TEXT    NOT NULL,
    display_name       TEXT    NOT NULL,
    receipt_name       TEXT    NOT NULL,
    product_scope      TEXT    NOT NULL,
    target_id          INTEGER,
    adjustment_type    TEXT    NOT NULL,
    adjustment_value   REAL    NOT NULL,
    is_active          INTEGER NOT NULL DEFAULT 1,
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_mg_discount_rule_group ON mg_discount_rule(marketing_group_id);

-- ── Stamp Activities (集章活动) ─────────────────────────────
CREATE TABLE stamp_activity (
    id                    INTEGER PRIMARY KEY,
    marketing_group_id    INTEGER NOT NULL REFERENCES marketing_group(id),
    name                  TEXT    NOT NULL,
    display_name          TEXT    NOT NULL,
    stamps_required       INTEGER NOT NULL,
    reward_quantity       INTEGER NOT NULL DEFAULT 1,
    reward_strategy       TEXT    NOT NULL DEFAULT 'ECONOMIZADOR',
    designated_product_id INTEGER,
    is_cyclic             INTEGER NOT NULL DEFAULT 1,
    is_active             INTEGER NOT NULL DEFAULT 1,
    created_at            INTEGER NOT NULL DEFAULT 0,
    updated_at            INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_stamp_activity_group ON stamp_activity(marketing_group_id);

-- ── Stamp Targets (集章条件目标) ────────────────────────────
CREATE TABLE stamp_target (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id) ON DELETE CASCADE,
    target_type       TEXT    NOT NULL,
    target_id         INTEGER NOT NULL
);
CREATE INDEX idx_stamp_target_activity ON stamp_target(stamp_activity_id);

-- ── Stamp Reward Targets (集章奖励目标) ─────────────────────
CREATE TABLE stamp_reward_target (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id) ON DELETE CASCADE,
    target_type       TEXT    NOT NULL,
    target_id         INTEGER NOT NULL
);
CREATE INDEX idx_stamp_reward_target_activity ON stamp_reward_target(stamp_activity_id);

-- ── Member Stamp Progress (会员集章进度) ────────────────────
CREATE TABLE member_stamp_progress (
    id                INTEGER PRIMARY KEY,
    member_id         INTEGER NOT NULL REFERENCES member(id),
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id),
    current_stamps    INTEGER NOT NULL DEFAULT 0,
    completed_cycles  INTEGER NOT NULL DEFAULT 0,
    last_stamp_at     INTEGER,
    updated_at        INTEGER NOT NULL DEFAULT 0,
    UNIQUE(member_id, stamp_activity_id)
);
```

**Step 3: Write DOWN migration**

写入 `0002_marketing_groups.down.sql`:

```sql
DROP TABLE IF EXISTS member_stamp_progress;
DROP TABLE IF EXISTS stamp_reward_target;
DROP TABLE IF EXISTS stamp_target;
DROP TABLE IF EXISTS stamp_activity;
DROP TABLE IF EXISTS mg_discount_rule;
DROP TABLE IF EXISTS member;
DROP TABLE IF EXISTS marketing_group;
```

**Step 4: Apply migration**

```bash
sqlx db reset -y --source edge-server/migrations
```

**Step 5: Update offline metadata**

```bash
cargo sqlx prepare --workspace
```

**Step 6: Verify**

```bash
cargo check --workspace
```

**Step 7: Commit**

```bash
git add edge-server/migrations/ .sqlx/
git commit -m "feat: add marketing group & member database schema"
```

---

## Phase 2: Shared Models

### Task 2: Marketing Group & MG Discount Rule models

**Files:**
- Create: `shared/src/models/marketing_group.rs`
- Modify: `shared/src/models/mod.rs`

**Step 1: Create marketing_group.rs**

遵循 `price_rule.rs` 模式。定义:

```rust
//! Marketing Group & MG Discount Rule Models

use serde::{Deserialize, Serialize};

// 复用 shared 已有枚举
use super::price_rule::{AdjustmentType, ProductScope};

/// Marketing Group entity (营销组 = 会员等级)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MarketingGroup {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub sort_order: i32,
    pub points_earn_rate: Option<f64>,
    pub points_per_unit: Option<f64>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingGroupCreate {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub points_earn_rate: Option<f64>,
    pub points_per_unit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingGroupUpdate {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
    pub points_earn_rate: Option<f64>,
    pub points_per_unit: Option<f64>,
    pub is_active: Option<bool>,
}

/// MG Discount Rule entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MgDiscountRule {
    pub id: i64,
    pub marketing_group_id: i64,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub product_scope: ProductScope,
    pub target_id: Option<i64>,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgDiscountRuleCreate {
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub product_scope: ProductScope,
    pub target_id: Option<i64>,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgDiscountRuleUpdate {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub receipt_name: Option<String>,
    pub product_scope: Option<ProductScope>,
    pub target_id: Option<i64>,
    pub adjustment_type: Option<AdjustmentType>,
    pub adjustment_value: Option<f64>,
    pub is_active: Option<bool>,
}
```

**Step 2: Register in mod.rs**

在 `shared/src/models/mod.rs` 添加 `pub mod marketing_group;` 和 `pub use marketing_group::*;`

**Step 3: Verify**

```bash
cargo check -p shared
```

**Step 4: Commit**

```bash
git add shared/src/models/
git commit -m "feat: add MarketingGroup and MgDiscountRule shared models"
```

### Task 3: Member model

**Files:**
- Create: `shared/src/models/member.rs`
- Modify: `shared/src/models/mod.rs`

**Step 1: Create member.rs**

```rust
//! Member Model

use serde::{Deserialize, Serialize};

/// Member entity (会员)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Member {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: i64,
    pub birthday: Option<String>,
    pub points_balance: i64,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberCreate {
    pub name: String,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: i64,
    pub birthday: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberUpdate {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: Option<i64>,
    pub birthday: Option<String>,
    pub notes: Option<String>,
    pub is_active: Option<bool>,
}

/// Member with marketing group info (for list/detail views)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MemberWithGroup {
    pub id: i64,
    pub name: String,
    pub phone: Option<String>,
    pub card_number: Option<String>,
    pub marketing_group_id: i64,
    pub marketing_group_name: String,
    pub birthday: Option<String>,
    pub points_balance: i64,
    pub notes: Option<String>,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
```

**Step 2: Register in mod.rs, verify, commit**

### Task 4: Stamp models

**Files:**
- Create: `shared/src/models/stamp.rs`
- Modify: `shared/src/models/mod.rs`

**Step 1: Create stamp.rs**

```rust
//! Stamp Activity & Progress Models

use serde::{Deserialize, Serialize};

/// Reward strategy for stamp activities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum RewardStrategy {
    /// Comp cheapest qualifying item in order (default)
    Economizador,
    /// Comp most expensive qualifying item in order
    Generoso,
    /// Comp a designated fixed product
    Designated,
}

/// Stamp target type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum StampTargetType {
    Category,
    Product,
}

/// Stamp Activity entity (集章活动)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StampActivity {
    pub id: i64,
    pub marketing_group_id: i64,
    pub name: String,
    pub display_name: String,
    pub stamps_required: i32,
    pub reward_quantity: i32,
    pub reward_strategy: RewardStrategy,
    pub designated_product_id: Option<i64>,
    pub is_cyclic: bool,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampActivityCreate {
    pub name: String,
    pub display_name: String,
    pub stamps_required: i32,
    pub reward_quantity: Option<i32>,
    pub reward_strategy: Option<RewardStrategy>,
    pub designated_product_id: Option<i64>,
    pub is_cyclic: Option<bool>,
    /// Stamp condition targets (which categories/products count)
    pub stamp_targets: Vec<StampTargetInput>,
    /// Reward targets (which categories/products can be comped)
    pub reward_targets: Vec<StampTargetInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampActivityUpdate {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub stamps_required: Option<i32>,
    pub reward_quantity: Option<i32>,
    pub reward_strategy: Option<RewardStrategy>,
    pub designated_product_id: Option<i64>,
    pub is_cyclic: Option<bool>,
    pub is_active: Option<bool>,
    /// If provided, replaces all stamp targets
    pub stamp_targets: Option<Vec<StampTargetInput>>,
    /// If provided, replaces all reward targets
    pub reward_targets: Option<Vec<StampTargetInput>>,
}

/// Input for stamp/reward target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampTargetInput {
    pub target_type: StampTargetType,
    pub target_id: i64,
}

/// Stamp target record (from DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StampTarget {
    pub id: i64,
    pub stamp_activity_id: i64,
    pub target_type: StampTargetType,
    pub target_id: i64,
}

/// Stamp reward target record (from DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct StampRewardTarget {
    pub id: i64,
    pub stamp_activity_id: i64,
    pub target_type: StampTargetType,
    pub target_id: i64,
}

/// Member stamp progress
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct MemberStampProgress {
    pub id: i64,
    pub member_id: i64,
    pub stamp_activity_id: i64,
    pub current_stamps: i32,
    pub completed_cycles: i32,
    pub last_stamp_at: Option<i64>,
    pub updated_at: i64,
}

/// Stamp activity with targets (for detail/config views)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StampActivityDetail {
    #[serde(flatten)]
    pub activity: StampActivity,
    pub stamp_targets: Vec<StampTarget>,
    pub reward_targets: Vec<StampRewardTarget>,
}

/// Member stamp progress with activity info (for member detail view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberStampProgressDetail {
    pub stamp_activity_id: i64,
    pub stamp_activity_name: String,
    pub stamps_required: i32,
    pub current_stamps: i32,
    pub completed_cycles: i32,
    pub is_redeemable: bool,
    pub is_cyclic: bool,
}
```

**Step 2: Register in mod.rs, verify, commit**

### Task 5: AppliedMgRule & OrderSnapshot extension

**Files:**
- Create: `shared/src/order/applied_mg_rule.rs`
- Modify: `shared/src/order/mod.rs`
- Modify: `shared/src/order/snapshot.rs` (add member fields + applied_mg_rules)
- Modify: `shared/src/order/event.rs` (add MemberLinked, MemberUnlinked, StampRedeemed)
- Modify: `shared/src/order/command.rs` (add LinkMember, UnlinkMember, RedeemStamp)

**Step 1: Create applied_mg_rule.rs**

```rust
//! Applied MG Rule (tracks MG discount applied to an order item)

use serde::{Deserialize, Serialize};
use super::super::models::price_rule::{AdjustmentType, ProductScope};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedMgRule {
    pub rule_id: i64,
    pub name: String,
    pub display_name: String,
    pub receipt_name: String,
    pub product_scope: ProductScope,
    pub adjustment_type: AdjustmentType,
    pub adjustment_value: f64,
    pub calculated_amount: f64,
    pub skipped: bool,
}
```

**Step 2: Extend OrderSnapshot**

在 `shared/src/order/snapshot.rs` 的 `OrderSnapshot` struct 中添加:

```rust
// Member info
#[serde(default, skip_serializing_if = "Option::is_none")]
pub member_id: Option<i64>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub member_name: Option<String>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub marketing_group_id: Option<i64>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub marketing_group_name: Option<String>,

// MG discount tracking
#[serde(default)]
pub applied_mg_rules: Vec<AppliedMgRule>,

// MG discount amounts (order level)
#[serde(default)]
pub mg_discount_amount: f64,
```

**Step 3: Extend OrderEventType & EventPayload**

在 `event.rs` 中 `OrderEventType` 添加:
```rust
MemberLinked,
MemberUnlinked,
StampRedeemed,
```

在 `EventPayload` 添加:
```rust
MemberLinked {
    member_id: i64,
    member_name: String,
    marketing_group_id: i64,
    marketing_group_name: String,
},
MemberUnlinked {
    previous_member_id: i64,
    previous_member_name: String,
},
StampRedeemed {
    stamp_activity_id: i64,
    stamp_activity_name: String,
    reward_item_id: i64,
    reward_strategy: String,
},
```

**Step 4: Extend OrderCommandPayload**

在 `command.rs` 中 `OrderCommandPayload` 添加:
```rust
LinkMember {
    order_id: String,
    member_id: i64,
},
UnlinkMember {
    order_id: String,
},
RedeemStamp {
    order_id: String,
    stamp_activity_id: i64,
    product_id: Option<i64>, // for Designated strategy
},
```

**Step 5: Register mod, verify, commit**

```bash
cargo check --workspace
git add shared/src/order/
git commit -m "feat: add member fields to OrderSnapshot and order events"
```

---

## Phase 3: Repository Layer

### Task 6: Marketing Group repository

**Files:**
- Create: `edge-server/src/db/repository/marketing_group.rs`
- Modify: `edge-server/src/db/repository/mod.rs`

CRUD for `marketing_group` table + `mg_discount_rule` + `stamp_activity` + targets.

Functions to implement:
```rust
// MarketingGroup
pub async fn find_all(pool) -> Vec<MarketingGroup>
pub async fn find_by_id(pool, id) -> Option<MarketingGroup>
pub async fn create(pool, data: MarketingGroupCreate) -> MarketingGroup
pub async fn update(pool, id, data: MarketingGroupUpdate) -> MarketingGroup
pub async fn delete(pool, id) -> bool  // soft delete via is_active

// MgDiscountRule
pub async fn find_rules_by_group(pool, group_id) -> Vec<MgDiscountRule>
pub async fn find_active_rules_by_group(pool, group_id) -> Vec<MgDiscountRule>
pub async fn create_rule(pool, group_id, data: MgDiscountRuleCreate) -> MgDiscountRule
pub async fn update_rule(pool, group_id, rule_id, data: MgDiscountRuleUpdate) -> MgDiscountRule
pub async fn delete_rule(pool, group_id, rule_id) -> bool

// StampActivity
pub async fn find_activities_by_group(pool, group_id) -> Vec<StampActivity>
pub async fn find_active_activities_by_group(pool, group_id) -> Vec<StampActivity>
pub async fn create_activity(pool, group_id, data: StampActivityCreate) -> StampActivityDetail
pub async fn update_activity(pool, group_id, activity_id, data: StampActivityUpdate) -> StampActivityDetail
pub async fn delete_activity(pool, group_id, activity_id) -> bool

// Stamp Targets (internal helpers)
async fn replace_stamp_targets(pool, activity_id, targets: &[StampTargetInput])
async fn replace_reward_targets(pool, activity_id, targets: &[StampTargetInput])
pub async fn find_stamp_targets(pool, activity_id) -> Vec<StampTarget>
pub async fn find_reward_targets(pool, activity_id) -> Vec<StampRewardTarget>
```

遵循 `tag.rs` / `price_rule.rs` 的 CRUD 模式。
`create_activity` 和 `update_activity` 需要在事务中同时处理 targets 表。

**Step: Implement, register in mod.rs, verify, commit**

### Task 7: Member repository

**Files:**
- Create: `edge-server/src/db/repository/member.rs`
- Modify: `edge-server/src/db/repository/mod.rs`

```rust
pub async fn find_all(pool) -> Vec<MemberWithGroup>
pub async fn find_by_id(pool, id) -> Option<MemberWithGroup>
pub async fn search(pool, query: &str) -> Vec<MemberWithGroup>  // 手机号/卡号/姓名模糊搜索
pub async fn create(pool, data: MemberCreate) -> MemberWithGroup
pub async fn update(pool, id, data: MemberUpdate) -> MemberWithGroup
pub async fn delete(pool, id) -> bool  // soft delete

// For order linking
pub async fn find_member_by_id(pool, id) -> Option<Member>  // 简单查询，不 join
```

`find_all` 和 `search` 使用 JOIN 查询 `marketing_group.display_name`:
```sql
SELECT m.*, mg.display_name as marketing_group_name
FROM member m
JOIN marketing_group mg ON m.marketing_group_id = mg.id
WHERE m.is_active = 1
```

`search` 使用 `WHERE m.phone LIKE ? OR m.card_number LIKE ? OR m.name LIKE ?`

**Step: Implement, register in mod.rs, verify, commit**

### Task 8: Stamp progress repository

**Files:**
- Create: `edge-server/src/db/repository/stamp.rs`
- Modify: `edge-server/src/db/repository/mod.rs`

```rust
/// Get all stamp progress for a member
pub async fn find_progress_by_member(pool, member_id) -> Vec<MemberStampProgress>

/// Get progress for a specific member + activity
pub async fn find_progress(pool, member_id, activity_id) -> Option<MemberStampProgress>

/// Get progress with activity detail for member (for display)
pub async fn find_progress_details_by_member(pool, member_id) -> Vec<MemberStampProgressDetail>

/// Add stamps (called when order completes)
pub async fn add_stamps(pool, member_id, activity_id, count: i32, timestamp: i64) -> MemberStampProgress

/// Redeem stamps (reset progress)
pub async fn redeem(pool, member_id, activity_id, is_cyclic: bool, timestamp: i64) -> MemberStampProgress

/// Initialize progress record if not exists
pub async fn ensure_progress(pool, member_id, activity_id) -> MemberStampProgress
```

**Step: Implement, register in mod.rs, verify, commit**

---

## Phase 4: Feature Gate & Permissions

### Task 9: Feature gate middleware

**Files:**
- Create: `edge-server/src/auth/feature_gate.rs`
- Modify: `edge-server/src/auth/mod.rs`

**Step 1: Create feature_gate.rs**

```rust
//! Feature Gate Middleware
//!
//! Gates API routes based on subscription features.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use crate::utils::AppError;
use shared::error::ErrorCode;

/// Create a feature gate middleware that checks SubscriptionInfo.features
pub fn require_feature(
    feature: &'static str,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> + Clone {
    move |req: Request, next: Next| {
        Box::pin(async move {
            // Get ServerState from request extensions
            let state = req.extensions().get::<crate::core::ServerState>()
                .cloned()
                .ok_or_else(|| AppError::internal("ServerState not found"))?;

            let has_feature = state.activation
                .has_feature(feature)
                .await;

            if !has_feature {
                return Err(AppError::with_message(
                    ErrorCode::FeatureNotAvailable,
                    format!("Feature '{}' requires Pro plan or above", feature),
                ));
            }

            Ok(next.run(req).await)
        })
    }
}
```

注意: 需要确认 `ActivationService` 上是否已有 `has_feature()` 方法。如果没有，需要在 `edge-server/src/services/activation.rs` 添加:

```rust
pub async fn has_feature(&self, feature: &str) -> bool {
    if let Some(sub) = self.get_subscription().await {
        sub.features.iter().any(|f| f == feature)
    } else {
        false
    }
}
```

同时在 `shared/src/error/codes.rs` 添加错误码:
```rust
FeatureNotAvailable = 3010,
```

**Step 2: Register in auth/mod.rs, verify, commit**

### Task 10: Add new permissions

**Files:**
- Modify: `edge-server/src/auth/permissions.rs`

**Step 1: Add permissions**

在 `ALL_PERMISSIONS` 数组中添加:
```rust
// === 营销与会员 (4) ===
"members:manage",           // 会员管理
"marketing_groups:manage",  // 营销组管理
"orders:link_member",       // 订单绑定/解绑会员
"orders:redeem_stamp",      // 集章兑换
```

**Step 2: Verify, commit**

---

## Phase 5: Marketing Engine

### Task 11: MG discount calculator

**Files:**
- Create: `edge-server/src/marketing/mod.rs`
- Create: `edge-server/src/marketing/mg_calculator.rs`
- Modify: `edge-server/src/lib.rs` (add `pub mod marketing;`)

**Step 1: Create mod.rs**

```rust
//! Marketing Engine
//!
//! Independent from pricing/ module.
//! Handles MG discount calculations and stamp tracking.

pub mod mg_calculator;
pub mod stamp_tracker;
```

**Step 2: Create mg_calculator.rs**

核心逻辑: 给定一组 `MgDiscountRule` 和商品信息，计算 MG 折扣。
所有规则全部叠乘（资本家模式），无排他/不可叠加逻辑。

```rust
//! MG Discount Calculator

use rust_decimal::prelude::*;
use shared::models::{MgDiscountRule, ProductScope, AdjustmentType};
use shared::order::AppliedMgRule;

pub struct MgCalculationResult {
    pub mg_discount: f64,
    pub applied_rules: Vec<AppliedMgRule>,
}

/// Calculate MG discount for a single item
pub fn calculate_mg_discount(
    base_price: f64,          // price after PriceRule discounts
    product_id: i64,
    category_id: Option<i64>,
    rules: &[MgDiscountRule],
) -> MgCalculationResult {
    // 1. Filter matching rules (active + product scope match)
    // 2. Apply all matched rules multiplicatively (capitalist mode)
    //    - Percentage: multiply (1 - rate/100)
    //    - FixedAmount: subtract directly
    // 3. Return total discount and list of applied rules
}

/// Check if a rule matches a product
fn matches_product(rule: &MgDiscountRule, product_id: i64, category_id: Option<i64>) -> bool {
    match rule.product_scope {
        ProductScope::Global => true,
        ProductScope::Product => rule.target_id == Some(product_id),
        ProductScope::Category => rule.target_id == category_id,
        _ => false, // Tag scope not used for MG rules
    }
}
```

参考 `edge-server/src/pricing/item_calculator.rs` 的计算模式，但简化（无排他/叠加控制）。
使用 `rust_decimal` 进行精确计算，最终转为 `f64`。

**Step 3: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::dec;

    #[test]
    fn test_calculate_mg_discount_single_percentage() { ... }

    #[test]
    fn test_calculate_mg_discount_stacking() { ... }

    #[test]
    fn test_calculate_mg_discount_fixed_amount() { ... }

    #[test]
    fn test_calculate_mg_discount_scope_filtering() { ... }

    #[test]
    fn test_calculate_mg_discount_no_matching_rules() { ... }
}
```

**Step 4: Verify, commit**

### Task 12: Stamp tracker

**Files:**
- Create: `edge-server/src/marketing/stamp_tracker.rs`

**Step 1: Create stamp_tracker.rs**

```rust
//! Stamp Tracker
//!
//! Handles stamp counting and redemption logic.

use shared::models::{
    StampActivity, StampTarget, StampRewardTarget, StampTargetType, RewardStrategy,
};
use shared::order::CartItemSnapshot;

/// Count how many stamps an order earns for a given activity
pub fn count_stamps_for_order(
    items: &[CartItemSnapshot],
    stamp_targets: &[StampTarget],
) -> i32 {
    // For each non-comp item, check if it matches any stamp target
    // Sum up quantities of matching items
    items.iter()
        .filter(|item| !item.is_comp && matches_stamp_target(item, stamp_targets))
        .map(|item| item.quantity)
        .sum()
}

/// Check if an item matches any stamp target
fn matches_stamp_target(item: &CartItemSnapshot, targets: &[StampTarget]) -> bool {
    targets.iter().any(|t| match t.target_type {
        StampTargetType::Product => t.target_id == item.product_id,
        StampTargetType::Category => Some(t.target_id) == item.category_id,
    })
}

/// Find the item to comp based on reward strategy
pub fn find_reward_item(
    items: &[CartItemSnapshot],
    reward_targets: &[StampRewardTarget],
    strategy: &RewardStrategy,
) -> Option<String> {
    // instance_id of the item to comp
    match strategy {
        RewardStrategy::Economizador => {
            // Find cheapest matching non-comp item
            items.iter()
                .filter(|item| !item.is_comp && matches_reward_target(item, reward_targets))
                .min_by(|a, b| a.unit_price.partial_cmp(&b.unit_price).unwrap())
                .map(|item| item.instance_id.clone())
        }
        RewardStrategy::Generoso => {
            // Find most expensive matching non-comp item
            items.iter()
                .filter(|item| !item.is_comp && matches_reward_target(item, reward_targets))
                .max_by(|a, b| a.unit_price.partial_cmp(&b.unit_price).unwrap())
                .map(|item| item.instance_id.clone())
        }
        RewardStrategy::Designated => {
            // Designated uses designated_product_id, handled at action level
            None
        }
    }
}

fn matches_reward_target(item: &CartItemSnapshot, targets: &[StampRewardTarget]) -> bool {
    targets.iter().any(|t| match t.target_type {
        StampTargetType::Product => t.target_id == item.product_id,
        StampTargetType::Category => Some(t.target_id) == item.category_id,
    })
}
```

**Step 2: Write tests, verify, commit**

---

## Phase 6: API Layer

### Task 13: Member API

**Files:**
- Create: `edge-server/src/api/members/mod.rs`
- Create: `edge-server/src/api/members/handler.rs`
- Modify: `edge-server/src/api/mod.rs`
- Modify: `edge-server/src/services/https.rs`

**Step 1: Create mod.rs (routes)**

遵循 `tables/mod.rs` 模式:
```rust
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/members", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/search", get(handler::search))
        .route("/{id}", get(handler::get_by_id));

    let manage_routes = Router::new()
        .route("/", post(handler::create))
        .route("/{id}", put(handler::update).delete(handler::delete))
        .layer(middleware::from_fn(require_permission("members:manage")));

    // TODO: wrap all routes with feature gate
    // .layer(middleware::from_fn(require_feature("marketing")))

    read_routes.merge(manage_routes)
}
```

**Step 2: Create handler.rs**

遵循 `tables/handler.rs` 模式:
- `list` - GET /api/members
- `search` - GET /api/members/search?q=xxx
- `get_by_id` - GET /api/members/:id (含集章进度)
- `create` - POST /api/members (+ audit_log + broadcast_sync)
- `update` - PUT /api/members/:id (+ audit_log + broadcast_sync)
- `delete` - DELETE /api/members/:id (soft delete, + audit_log + broadcast_sync)

`get_by_id` 额外查询 `stamp::find_progress_details_by_member` 返回集章进度。

**Step 3: Add AuditAction variants**

在 `edge-server/src/audit/types.rs` 添加:
```rust
// ═══ 会员 ═══
MemberCreated,
MemberUpdated,
MemberDeleted,
```

**Step 4: Register in api/mod.rs and https.rs, verify, commit**

### Task 14: Marketing Group API

**Files:**
- Create: `edge-server/src/api/marketing_groups/mod.rs`
- Create: `edge-server/src/api/marketing_groups/handler.rs`
- Modify: `edge-server/src/api/mod.rs`
- Modify: `edge-server/src/services/https.rs`

**Step 1: Create mod.rs (routes)**

```rust
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/marketing-groups", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id));

    let manage_routes = Router::new()
        .route("/", post(handler::create))
        .route("/{id}", put(handler::update).delete(handler::delete))
        // Discount rules
        .route("/{id}/discount-rules", post(handler::create_rule))
        .route("/{id}/discount-rules/{rule_id}", put(handler::update_rule).delete(handler::delete_rule))
        // Stamp activities
        .route("/{id}/stamp-activities", post(handler::create_activity))
        .route("/{id}/stamp-activities/{activity_id}", put(handler::update_activity).delete(handler::delete_activity))
        .layer(middleware::from_fn(require_permission("marketing_groups:manage")));

    read_routes.merge(manage_routes)
}
```

**Step 2: Create handler.rs**

CRUD handlers for MG + nested discount rules + stamp activities.

`get_by_id` 返回完整 MG 详情:
```rust
#[derive(Serialize)]
pub struct MarketingGroupDetail {
    #[serde(flatten)]
    pub group: MarketingGroup,
    pub discount_rules: Vec<MgDiscountRule>,
    pub stamp_activities: Vec<StampActivityDetail>,
}
```

**Step 3: Add AuditAction variants**

```rust
// ═══ 营销组 ═══
MarketingGroupCreated,
MarketingGroupUpdated,
MarketingGroupDeleted,
```

**Step 4: Register, verify, commit**

---

## Phase 7: Order Integration

### Task 15: Order actions — LinkMember & UnlinkMember

**Files:**
- Create: `edge-server/src/orders/actions/link_member.rs`
- Create: `edge-server/src/orders/actions/unlink_member.rs`
- Create: `edge-server/src/orders/appliers/member.rs`
- Modify: `edge-server/src/orders/actions/mod.rs` (register)
- Modify: `edge-server/src/orders/appliers/mod.rs` (register)

**Step 1: Create link_member.rs**

```rust
pub struct LinkMemberAction {
    pub order_id: String,
    pub member_id: i64,
    // Injected by OrdersManager
    pub member_name: String,
    pub marketing_group_id: i64,
    pub marketing_group_name: String,
    pub mg_rules: Vec<MgDiscountRule>,
}

#[async_trait]
impl CommandHandler for LinkMemberAction {
    async fn execute(&self, ctx, metadata) -> Result<Vec<OrderEvent>, OrderError> {
        let snapshot = ctx.get_snapshot(&self.order_id)?;
        // 1. Validate: order must be Active
        // 2. Set member info on snapshot
        // 3. Recalculate all item prices with MG rules
        // 4. Emit MemberLinked event
    }
}
```

**Step 2: Create unlink_member.rs**

```rust
pub struct UnlinkMemberAction {
    pub order_id: String,
}

#[async_trait]
impl CommandHandler for UnlinkMemberAction {
    async fn execute(&self, ctx, metadata) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Validate: order has a member linked
        // 2. Clear member info from snapshot
        // 3. Recalculate prices without MG rules
        // 4. Emit MemberUnlinked event
    }
}
```

**Step 3: Create appliers/member.rs**

```rust
pub struct MemberLinkedApplier;
impl EventApplier for MemberLinkedApplier {
    fn apply(&self, snapshot, event) {
        if let EventPayload::MemberLinked { member_id, member_name, marketing_group_id, marketing_group_name } = &event.payload {
            snapshot.member_id = Some(*member_id);
            snapshot.member_name = Some(member_name.clone());
            snapshot.marketing_group_id = Some(*marketing_group_id);
            snapshot.marketing_group_name = Some(marketing_group_name.clone());
        }
    }
}

pub struct MemberUnlinkedApplier;
impl EventApplier for MemberUnlinkedApplier {
    fn apply(&self, snapshot, event) {
        snapshot.member_id = None;
        snapshot.member_name = None;
        snapshot.marketing_group_id = None;
        snapshot.marketing_group_name = None;
        snapshot.applied_mg_rules.clear();
        snapshot.mg_discount_amount = 0.0;
    }
}
```

**Step 4: Register in actions/mod.rs and appliers/mod.rs**

添加 `CommandAction::LinkMember` 和 `CommandAction::UnlinkMember` variants。
在 `From<&OrderCommand>` impl 中添加对应的 match 分支。

**Step 5: Update OrdersManager**

在 `edge-server/src/orders/manager/mod.rs` 中，处理 `LinkMember` 命令时:
1. 查询 member 信息 (name, marketing_group_id)
2. 查询 MG 的 active discount rules
3. 注入到 `LinkMemberAction`
4. 缓存 MG rules (类似 PriceRule 缓存)

**Step 6: Verify, commit**

### Task 16: Integrate MG discounts into price calculation

**Files:**
- Modify: `edge-server/src/pricing/item_calculator.rs` (or create new integration point)
- Modify: `edge-server/src/orders/money/mod.rs`

**关键决策**: MG 折扣独立计算，在 `recalculate_totals` 中作为新步骤应用。

在 `orders/money/mod.rs` 的 `recalculate_totals()` 中:
1. 现有逻辑计算 PriceRule 折扣后的 `subtotal`
2. 新增: 如果订单有 `applied_mg_rules`，计算 MG 折扣并从 subtotal 中扣除
3. MG 折扣与 PriceRule 折扣叠乘

**Step: Implement, test, verify, commit**

### Task 17: Stamp tracking on order completion

**Files:**
- Modify: `edge-server/src/orders/actions/complete_order.rs`
- Or: Hook into `EventRouter` on OrderCompleted event

**关键决策**: 集章在订单完成时写入。最好在 `EventRouter` 层处理（类似 Archive），避免在 `complete_order.rs` 中做 I/O。

在 `EventRouter` 或 `OrdersManager` 的 post-completion hook 中:
1. 检查订单是否有 member_id
2. 加载该 MG 的活跃 stamp activities
3. 遍历订单商品，调用 `stamp_tracker::count_stamps_for_order`
4. 调用 `stamp::add_stamps` 更新进度
5. 如果集满，通知前端（通过 broadcast）

**Step: Implement, verify, commit**

### Task 18: Stamp redemption command

**Files:**
- Create: `edge-server/src/orders/actions/redeem_stamp.rs`
- Create: `edge-server/src/orders/appliers/stamp.rs`
- Modify: `edge-server/src/orders/actions/mod.rs`
- Modify: `edge-server/src/orders/appliers/mod.rs`

```rust
pub struct RedeemStampAction {
    pub order_id: String,
    pub stamp_activity_id: i64,
    pub product_id: Option<i64>, // for Designated
    // Injected
    pub activity: StampActivity,
    pub stamp_targets: Vec<StampTarget>,
    pub reward_targets: Vec<StampRewardTarget>,
}

impl CommandHandler for RedeemStampAction {
    async fn execute(&self, ctx, metadata) -> Result<Vec<OrderEvent>, OrderError> {
        let snapshot = ctx.get_snapshot(&self.order_id)?;
        // 1. Validate: order has member, stamps are sufficient
        // 2. Find reward item based on strategy
        //    - Economizador/Generoso: use stamp_tracker::find_reward_item
        //    - Designated: use activity.designated_product_id
        // 3. Comp the selected item (set unit_price = 0, is_comp = true)
        // 4. Update stamp progress (reset or mark complete)
        // 5. Emit StampRedeemed event
    }
}
```

**Step: Implement, verify, commit**

---

## Phase 8: Frontend Types

### Task 19: TypeScript type definitions

**Files:**
- Modify: `red_coral/src/core/domain/types/api/models.ts`

添加:
```typescript
// ============ Marketing Group ============

export interface MarketingGroup {
    id: number;
    name: string;
    display_name: string;
    description: string | null;
    sort_order: number;
    points_earn_rate: number | null;
    points_per_unit: number | null;
    is_active: boolean;
    created_at: number;
    updated_at: number;
}

export interface MarketingGroupCreate { ... }
export interface MarketingGroupUpdate { ... }

export interface MgDiscountRule {
    id: number;
    marketing_group_id: number;
    name: string;
    display_name: string;
    receipt_name: string;
    product_scope: ProductScope;
    target_id: number | null;
    adjustment_type: AdjustmentType;
    adjustment_value: number;
    is_active: boolean;
    created_at: number;
    updated_at: number;
}

// ============ Member ============

export interface Member {
    id: number;
    name: string;
    phone: string | null;
    card_number: string | null;
    marketing_group_id: number;
    birthday: string | null;
    points_balance: number;
    notes: string | null;
    is_active: boolean;
    created_at: number;
    updated_at: number;
}

export interface MemberWithGroup extends Member {
    marketing_group_name: string;
}

// ============ Stamp ============

export type RewardStrategy = 'ECONOMIZADOR' | 'GENEROSO' | 'DESIGNATED';
export type StampTargetType = 'CATEGORY' | 'PRODUCT';

export interface StampActivity { ... }
export interface StampTarget { ... }
export interface StampRewardTarget { ... }
export interface MemberStampProgress { ... }
export interface MemberStampProgressDetail { ... }

// ============ Applied MG Rule ============

export interface AppliedMgRule {
    rule_id: number;
    name: string;
    display_name: string;
    receipt_name: string;
    product_scope: ProductScope;
    adjustment_type: AdjustmentType;
    adjustment_value: number;
    calculated_amount: number;
    skipped: boolean;
}
```

同时在 OrderSnapshot 的 TypeScript 类型中添加 member 字段。

**Step: Implement, tsc --noEmit, commit**

---

## Phase 9: Frontend Pages (概要)

> 前端页面实现较复杂，以下为概要任务，每个任务需要根据现有前端模式进一步细化。

### Task 20: Member management page

- 会员列表（搜索框 + 表格）
- 创建/编辑会员模态框
- 查看会员详情（含集章进度）

### Task 21: Marketing Group management page

- MG 列表
- 创建/编辑 MG
- MG 详情页：折扣规则配置 + 集章活动配置
- 集章活动配置：stamp targets + reward targets + reward strategy

### Task 22: Order sidebar member panel

- 会员搜索（手机号/卡号/姓名）
- 绑定/解绑会员按钮
- 显示当前会员权益（MG 名称、折扣规则）
- 显示集章进度
- 兑换按钮（集满时启用）

### Task 23: Feature gate UI

- 检查 `features.includes("marketing")`
- 非 Pro 用户: 隐藏会员/MG 菜单入口，或显示「Pro plan 专属」提示
- 通过 `PermissionGate` 控制管理权限

---

## Verification Checklist

每个 Phase 完成后验证:

```bash
# Rust 编译 + lint
cargo check --workspace
cargo clippy --workspace

# Rust 测试
cargo test --workspace --lib

# TypeScript 类型检查
cd red_coral && npx tsc --noEmit

# SQLx 离线元数据
cargo sqlx prepare --workspace
```

## File Summary

### 新建文件

| File | Purpose |
|------|---------|
| `edge-server/migrations/0002_marketing_groups.{up,down}.sql` | DB schema |
| `shared/src/models/marketing_group.rs` | MG + MgDiscountRule models |
| `shared/src/models/member.rs` | Member model |
| `shared/src/models/stamp.rs` | Stamp models |
| `shared/src/order/applied_mg_rule.rs` | AppliedMgRule |
| `edge-server/src/db/repository/marketing_group.rs` | MG repository |
| `edge-server/src/db/repository/member.rs` | Member repository |
| `edge-server/src/db/repository/stamp.rs` | Stamp progress repository |
| `edge-server/src/auth/feature_gate.rs` | Feature gate middleware |
| `edge-server/src/marketing/mod.rs` | Marketing engine module |
| `edge-server/src/marketing/mg_calculator.rs` | MG discount calculator |
| `edge-server/src/marketing/stamp_tracker.rs` | Stamp tracker |
| `edge-server/src/api/members/mod.rs` | Member API routes |
| `edge-server/src/api/members/handler.rs` | Member API handlers |
| `edge-server/src/api/marketing_groups/mod.rs` | MG API routes |
| `edge-server/src/api/marketing_groups/handler.rs` | MG API handlers |
| `edge-server/src/orders/actions/link_member.rs` | LinkMember command |
| `edge-server/src/orders/actions/unlink_member.rs` | UnlinkMember command |
| `edge-server/src/orders/actions/redeem_stamp.rs` | RedeemStamp command |
| `edge-server/src/orders/appliers/member.rs` | Member event appliers |
| `edge-server/src/orders/appliers/stamp.rs` | Stamp event applier |

### 修改文件

| File | Changes |
|------|---------|
| `shared/src/models/mod.rs` | Add 3 modules |
| `shared/src/order/mod.rs` | Add applied_mg_rule module |
| `shared/src/order/snapshot.rs` | Add member fields + applied_mg_rules |
| `shared/src/order/event.rs` | Add 3 event types |
| `shared/src/order/command.rs` | Add 3 command types |
| `shared/src/error/codes.rs` | Add FeatureNotAvailable |
| `edge-server/src/lib.rs` | Add marketing module |
| `edge-server/src/db/repository/mod.rs` | Add 3 repositories |
| `edge-server/src/api/mod.rs` | Add 2 API modules |
| `edge-server/src/services/https.rs` | Register 2 routers |
| `edge-server/src/auth/mod.rs` | Add feature_gate |
| `edge-server/src/auth/permissions.rs` | Add 4 permissions |
| `edge-server/src/audit/types.rs` | Add 6 audit actions |
| `edge-server/src/orders/actions/mod.rs` | Add 3 actions |
| `edge-server/src/orders/appliers/mod.rs` | Add 3 appliers |
| `edge-server/src/orders/money/mod.rs` | MG discount integration |
| `red_coral/src/core/domain/types/api/models.ts` | Add TS types |
