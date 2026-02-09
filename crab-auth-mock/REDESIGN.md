# crab-auth 重设计方案

## 概述

- 现有 `crab-auth` → 改名为 **`crab-auth-mock`**，保留本地开发 / CI 测试用
- 新建 **`crab-auth`**，生产级认证服务（PG + 设备 quota + 证书签发 + 订阅校验）

两者暴露**相同的 API 接口**，edge-server 通过 `auth_url` 配置切换：

```
开发: auth_url = "http://localhost:3001"   → crab-auth-mock (内存存储)
生产: auth_url = "https://auth.crab.io"    → crab-auth (PG)
```

### crab-auth 职责边界（只做这些）

- 设备激活（验证凭据 + quota 检查 + 签发证书）
- 订阅校验（读 PG → 签名 SubscriptionInfo → 返回给 edge-server）
- Binding refresh（检查设备状态 + 重新签名）
- PKI 管理（Root CA / Tenant CA 生命周期）

### 不做的事（由其他服务负责）

| 职责 | 由谁负责 |
|------|----------|
| 租户 CRUD | SaaS 管理平台 → 写 PG `tenants` |
| Stripe Webhook 处理 | Webhook 处理服务 → 写 PG `subscriptions` |
| 设备解绑/撤销 | 管理平台 → 直接写 PG `activations.status` |
| 管理后台 UI | 独立前端 → 调管理平台 API |

## 决策记录

| 项目 | 决策 |
|------|------|
| 现有 crab-auth | 改名 `crab-auth-mock`，保留 |
| 新 crab-auth | 生产版，专注证书 + 订阅 |
| 持久化存储 | PostgreSQL（与 SaaS 管理平台共享） |
| 订阅管理 | Stripe → PG，crab-auth 只读 |
| 设备限制 | 激活总数，quota 满时支持替换 |
| 租户管理 | 外部平台创建，crab-auth 只读 |
| 部署目标 | AWS ECS Fargate |
| 认证方式 | username/password（查 PG） |

## 架构

```
┌──────────────────┐     ┌─────────────┐
│ SaaS 管理平台     │────▶│             │
│ (租户/用户 CRUD)  │     │ PostgreSQL  │
└──────────────────┘     │             │
┌──────────────────┐     │  tenants    │
│ Stripe Webhooks  │────▶│  subscriptions│
│ (订阅状态同步)    │     │  activations │
└──────────────────┘     │             │
                         └──────┬──────┘
                                │
          ┌─────────────────────┤
          │ 读 tenants          │ 读 subscriptions
          │ 读/写 activations   │
          ▼                     │
   ┌─────────────┐             │
   │  crab-auth  │ + auth_storage/ (CA 证书)
   │  (Fargate)  │
   └──────┬──────┘
          │ 激活 / 证书 / 订阅校验
          ├───────────┬───────────┐
          ▼           ▼           ▼
    edge-server  edge-server  edge-server
    (门店A)      (门店B)      (门店C)
```

### crab-auth-mock vs crab-auth

| | crab-auth-mock (开发) | crab-auth (生产) |
|---|---|---|
| 存储 | 内存 HashMap + 文件 | PostgreSQL + 文件 |
| 用户 | 硬编码测试用户 | PG `tenants` 表 |
| 订阅 | `match tenant_id` Mock | PG `subscriptions` (Stripe) |
| 设备追踪 | 无 | PG `activations` + quota |
| 撤销 | 内存 RevocationStore | PG `activations.status` |
| 部署 | 本地 `cargo run` | AWS Fargate |
| 依赖 | shared, crab-cert | shared, crab-cert, sqlx(pg) |

## crab-auth 项目结构

```
crab-auth/
├── Cargo.toml
├── CLAUDE.md
├── migrations/             # sqlx migrations (仅 activations 表)
│   └── 0001_activations.sql
└── src/
    ├── main.rs             # 入口 + PG 初始化
    ├── config.rs           # 环境变量配置
    ├── state.rs            # AppState { db: PgPool, auth_storage }
    ├── api/
    │   ├── mod.rs          # Router 定义
    │   ├── activate.rs     # POST /api/server/activate
    │   ├── subscription.rs # POST /api/tenant/subscription
    │   ├── binding.rs      # POST /api/binding/refresh
    │   └── pki.rs          # GET /pki/root_ca
    └── db/
        ├── mod.rs
        ├── tenants.rs      # 查询 tenants (只读)
        ├── subscriptions.rs # 查询 subscriptions (只读)
        └── activations.rs  # CRUD activations (读写)
```

## 数据库设计

### tenants（外部平台写，crab-auth 只读）

```sql
CREATE TABLE tenants (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    hashed_password TEXT NOT NULL,                   -- argon2 哈希
    status          TEXT NOT NULL DEFAULT 'active',  -- active / suspended / deleted
    created_at      BIGINT NOT NULL,
    updated_at      BIGINT NOT NULL
);
```

### subscriptions（Stripe webhook 写，crab-auth 只读）

```sql
CREATE TABLE subscriptions (
    id                      TEXT PRIMARY KEY,
    tenant_id               TEXT NOT NULL REFERENCES tenants(id),
    stripe_subscription_id  TEXT,
    status                  TEXT NOT NULL,           -- active / past_due / canceled / unpaid / expired / inactive
    plan                    TEXT NOT NULL,           -- basic / pro / enterprise
    max_edge_servers        INTEGER NOT NULL DEFAULT 1,
    features                TEXT[] NOT NULL DEFAULT '{}',
    current_period_start    BIGINT,
    current_period_end      BIGINT,
    created_at              BIGINT NOT NULL,
    updated_at              BIGINT NOT NULL
);

CREATE INDEX idx_subscriptions_tenant ON subscriptions(tenant_id);
```

### activations（crab-auth 读写）

```sql
CREATE TABLE activations (
    entity_id         TEXT PRIMARY KEY,                -- edge-server-{uuid}
    tenant_id         TEXT NOT NULL REFERENCES tenants(id),
    device_id         TEXT NOT NULL,
    fingerprint       TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'active',  -- active / deactivated / replaced / revoked
    activated_at      BIGINT NOT NULL,
    deactivated_at    BIGINT,
    replaced_by       TEXT REFERENCES activations(entity_id),
    last_refreshed_at BIGINT,

    UNIQUE(tenant_id, device_id)
);

CREATE INDEX idx_activations_tenant_status ON activations(tenant_id, status);
```

### activation 状态机

```
                  ┌──────────┐
     新设备激活 ──▶│  active  │◀── 重新激活（同设备）
                  └────┬─────┘
                       │
          ┌────────────┼────────────┐
          ▼            ▼            ▼
   ┌─────────────┐ ┌──────────┐ ┌─────────┐
   │ deactivated │ │ replaced │ │ revoked │
   │ (管理平台   │ │ (被新设备 │ │ (管理平台│
   │  操作)      │ │  挤掉)   │ │  操作)  │
   └──────┬──────┘ └──────────┘ └─────────┘
          │
          ▼
      可重新激活
```

> 注意: deactivated / revoked 状态由管理平台直接写 PG，crab-auth 只读这些状态。
> replaced 状态由 crab-auth 在设备替换时写入。

## 激活流程

### 正常激活（quota 未满）

```
POST /api/server/activate
{ "username": "...", "password": "...", "device_id": "hw-abc123" }
```

```
1. 查 tenants: 验证凭据，获取 tenant_id
   → 失败 → 400 "Invalid credentials"

2. 查 subscriptions: 获取当前订阅
   → status blocked → 403 "Subscription inactive"
   → 无记录 → 403 "No active subscription"

3. 查 activations: 检查 quota
   SELECT COUNT(*) FROM activations WHERE tenant_id = ? AND status = 'active'
   → count < max_edge_servers → 继续

4. 检查同设备重激活
   SELECT * FROM activations WHERE tenant_id = ? AND device_id = ?
   → 存在且 active → 复用 entity_id，续签证书
   → 存在且 deactivated → 重新激活 (UPDATE status = 'active')
   → 不存在 → 新设备

5. 签发证书 + 写入 activations → 返回 ActivationResponse
```

### Quota 已满 + 设备替换

```
POST /api/server/activate
{ ..., "replace_entity_id": "edge-server-old-xxx" }
```

```
quota 已满 + 无 replace_entity_id:
  → 返回 409 + QuotaInfo { max_edge_servers, active_count, active_devices }
  → 前端展示设备列表，用户选择替换

quota 已满 + 带 replace_entity_id:
  → 验证 replace_entity_id 属于该 tenant 且 active
  → UPDATE status = 'replaced', replaced_by = 新 entity_id
  → 继续激活新设备
```

### 被替换设备的处理

```
POST /api/binding/refresh → crab-auth 检查 activations.status
  → status != 'active' → 返回 403 { "error": "device_replaced" }
  → edge-server 进入 unbound 状态
```

## API 端点

### crab-auth 端点（与 crab-auth-mock 接口一致）

| 端点 | 说明 |
|------|------|
| `POST /api/server/activate` | 设备激活（PG 验证 + quota + 替换） |
| `POST /api/tenant/subscription` | 订阅状态查询（PG 读取 + 签名） |
| `POST /api/binding/refresh` | 刷新 binding（检查 activations.status） |
| `GET /pki/root_ca` | 获取 Root CA 证书 |

> 无管理 API — 设备管理由管理平台直接操作 PG。

## shared 类型变更

### ActivateRequest

```rust
pub struct ActivateRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_entity_id: Option<String>,  // 新增
}
```

### ActivationResponse

```rust
pub struct ActivationResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<ActivationData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_info: Option<QuotaInfo>,  // 新增
}

pub struct QuotaInfo {
    pub max_edge_servers: u32,
    pub active_count: u32,
    pub active_devices: Vec<ActiveDevice>,
}

pub struct ActiveDevice {
    pub entity_id: String,
    pub device_id: String,
    pub activated_at: i64,
    pub last_refreshed_at: Option<i64>,
}
```

> crab-auth-mock 忽略 `replace_entity_id`，`quota_info` 始终 None。
> 新字段都是 Option，向后兼容。

## edge-server 影响

改动最小：

1. binding refresh 处理新错误码 `device_replaced` → 进入 unbound 状态
2. ActivateRequest 加 `replace_entity_id`（Optional，不影响现有流程）
3. 其余逻辑不变

## 前端影响

1. ActivateRequest 加 `replace_entity_id`
2. 处理 quota 已满 → 展示设备列表 → 用户选择替换
3. 处理 `device_replaced` 状态 → 提示"设备已被替换"

## 证书存储

CA 证书继续用文件系统 (`auth_storage/`)：
- 部署时挂载 EFS + KMS 加密

## AWS 部署架构

```
                    Internet
                       │
                 ┌─────▼─────┐
                 │ Route 53  │  auth.crab.io
                 └─────┬─────┘
                       │
                 ┌─────▼─────┐
                 │   ALB     │  ACM 证书 (HTTPS)
                 └─────┬─────┘
                       │
┌─ VPC ────────────────┼────────────────────────┐
│                      │                         │
│  ┌───────────────────▼──────────────────────┐ │
│  │          ECS Fargate                      │ │
│  │          crab-auth                        │ │
│  │  Secrets Manager → DB_URL                 │ │
│  └──────┬──────────────────┬────────────────┘ │
│         │                  │                   │
│  ┌──────▼──────┐   ┌──────▼──────┐            │
│  │ Aurora v2   │   │ EFS + KMS   │            │
│  │ (PG)        │   │ auth_storage │            │
│  └─────────────┘   └─────────────┘            │
└────────────────────────────────────────────────┘
```

## 配置

```bash
DATABASE_URL=postgres://...
AUTH_STORAGE_PATH=/mnt/efs/auth_storage
PORT=3001
RUST_LOG=crab_auth=info
```

## 实现顺序

1. 现有 `crab-auth` → 改名 `crab-auth-mock`（目录 + Cargo.toml + workspace）
2. 新建 `crab-auth`（Cargo.toml + 基础结构）
3. `shared` 类型更新（ActivateRequest + ActivationResponse + QuotaInfo）
4. 数据库连接 + migrations（activations 表）
5. 实现 `activate()` — PG 查询 + quota + 替换 + 签发证书
6. 实现 `get_subscription_status()` — PG 读取 + 签名
7. 实现 `refresh_binding()` — 检查 activations.status
8. `GET /pki/root_ca`（复用 crab-cert）
9. edge-server binding refresh 错误处理（device_replaced）
10. 前端激活页面更新
