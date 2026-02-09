# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Crab Auth

认证服务器 — PostgreSQL 持久化 + PKI 证书管理 + 订阅校验 + P12 证书托管。

**云端唯一服务**，所有业务逻辑和数据都在餐厅本地 edge-server 运行。crab-auth 仅负责：
- 签发证书（低频，仅设备激活时）
- 订阅状态签名（每天/每周同步）
- P12 证书托管（Verifactu 电子签名合规）
- Root CA 私钥保管（最敏感资产）

本地开发请用 `crab-auth-mock`。

## 命令

```bash
cargo check -p crab-auth
cargo build -p crab-auth
```

## 依赖

| 组件 | 说明 |
|------|------|
| **PostgreSQL** | 租户、订阅、激活记录、P12 元数据 |
| **AWS S3** | .p12 证书文件存储 (SSE-KMS 加密) |
| **AWS Secrets Manager** | Root CA / Tenant CA 证书私钥 |

## 环境变量

| 变量 | 必须 | 默认值 | 说明 |
|------|------|--------|------|
| `DATABASE_URL` | **是** | - | PostgreSQL 连接字符串 |
| `PORT` | 否 | `3001` | HTTP 监听端口 |
| `P12_S3_BUCKET` | 否 | `crab-tenant-certificates` | S3 存储桶 |
| `P12_KMS_KEY_ID` | 否 | - | KMS Key ID (SSE-KMS) |

## 模块结构

```
src/
├── main.rs         # 入口 (PG 连接 + 迁移 + AWS SDK 初始化)
├── config.rs       # Config (从环境变量读取)
├── state.rs        # AppState (PgPool + CaStore + S3)
├── api/            # HTTP 路由 (Axum)
│   ├── mod.rs          # Router 定义
│   ├── activate.rs     # POST /api/server/activate (设备激活)
│   ├── subscription.rs # POST /api/tenant/subscription (订阅查询)
│   ├── binding.rs      # POST /api/binding/refresh (Binding 刷新)
│   ├── p12.rs          # POST /api/p12/upload (P12 证书上传)
│   └── pki.rs          # GET /pki/root_ca (Root CA 证书)
└── db/             # 数据访问层 (sqlx)
    ├── mod.rs
    ├── tenants.rs      # 租户认证 (argon2 密码校验)
    ├── subscriptions.rs# 订阅查询 (只读)
    ├── activations.rs  # 激活记录 CRUD
    └── p12.rs          # P12 证书元数据
```

## API 端点

| 端点 | 方法 | 用途 |
|------|------|------|
| `/api/server/activate` | POST | 设备激活 (认证 + 签发证书 + 返回订阅) |
| `/api/tenant/subscription` | POST | 查询/刷新订阅状态 (签名后返回) |
| `/api/binding/refresh` | POST | 刷新 SignedBinding (更新 last_verified_at) |
| `/api/p12/upload` | POST | 上传 .p12 证书 (Verifactu 电子签名) |
| `/pki/root_ca` | GET | 获取 Root CA 证书 |

## 数据库表

crab-auth **只拥有** `activations` 和 `p12_certificates` 表。
`tenants` 和 `subscriptions` 表由外部 SaaS 管理平台创建和维护 (Stripe webhook 等)，crab-auth 只读。

| 表 | 权限 | 说明 |
|-----|------|------|
| `activations` | 读写 | 设备激活记录 (entity_id, device_id, status) |
| `p12_certificates` | 读写 | P12 证书元数据 (S3 key, password, fingerprint) |
| `tenants` | 只读 | 租户信息 (id, name, hashed_password, status) |
| `subscriptions` | 只读 | 订阅信息 (plan, status, max_edge_servers, features) |

## 激活流程

```
1. Edge-server 发送 ActivationRequest (username, password, device_id)
2. 验证租户凭据 (argon2 密码校验)
3. 查询订阅状态 + Quota 检查 (max_edge_servers)
4. 如设备已存在 → 复用/替换旧激活记录
5. 签发 Entity Cert (Tenant CA 签名)
6. 生成 SignedBinding (硬件绑定 + 时钟篡改检测)
7. 签名 SubscriptionInfo (Tenant CA)
8. 返回: ActivationResponse { data: ActivationData, quota_info }
```

## 证书层级

```
Root CA (Secrets Manager: crab-auth/root-ca)
  └── Tenant CA (Secrets Manager: crab-auth/tenant/<tenant_id>)
        └── Entity Cert (签发给 edge-server, 含 device_id 绑定)
```

## 共享类型

使用 `shared::activation` 中的统一类型:
- `ActivationResponse` / `ActivationData`
- `SignedBinding` (含签名验证和时钟篡改检测)
- `SubscriptionInfo` (含签名，`last_checked_at` 服务端设为 `0`，由客户端本地更新)
- `SubscriptionStatus` / `PlanType` / `QuotaInfo`

## 部署架构

### 系统定位

```
crab-auth (云端, 唯一)          edge-server (餐厅本地, 每店一个)
├── PKI 根证书管理              ├── 完全自治 (离线可跑 10 天)
├── 设备激活 (一次性)           ├── 所有业务数据 (SQLite + redb)
├── 订阅签名 (低频)            └── 与 crab-auth 通信: 极低频
└── P12 托管 (合规)                (启动 + 每天 1-2 次)
```

### 流量特征

- **极低频**: 每餐厅每天 1-2 次请求（订阅同步 + Binding 刷新）
- **激活**: 一次性操作（开店/换设备时）
- **无实时性要求**: 所有调用方都容忍 1-2 秒延迟

### 部署: AWS Lambda + Secrets Manager

CA 私钥已迁移到 Secrets Manager，支持纯 serverless 部署:

| 组件 | 费用 |
|------|------|
| Lambda | $0（免费层内） |
| Secrets Manager | ~$1.20/月 |
| RDS db.t4g.micro | ~$12/月 |
| S3 (P12 证书) | $0.几 |
| **总计** | **~$13/月** |

### 安全要求

- **Root CA 私钥**: 最高机密，泄露等于整个 PKI 信任链崩塌
- **crab-auth 必须与普通网站隔离**: 不混部
- **RDS 放私有子网**: 外网不可达
- **P12 使用 SSE-KMS 加密存储**

### 离线容忍度

| 场景 | 影响 |
|------|------|
| crab-auth 宕机 < 10 天 | 无影响，edge-server 用缓存的签名 |
| crab-auth 宕机 > 10 天 | edge-server 订阅签名过期，进入阻止状态 |
| 首次激活 | 必须联网，无法离线完成 |

## 响应语言

使用中文回答。
