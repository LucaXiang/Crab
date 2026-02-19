# Design: 合并 crab-auth 到 crab-cloud

**日期**: 2026-02-18
**状态**: 待审批

## 动机

1. crab-auth (Lambda) 访问 EC2 PostgreSQL 需要 VPC Endpoints (~$14.4/月)
2. 两个服务共享同一个 PostgreSQL、同一个 Secrets Manager CaStore
3. 代码重复：CaStore、租户认证 (argon2)、AWS SDK 初始化
4. crab-cloud 已持有 Root CA 证书（mTLS 验证用），安全隔离名存实亡

## 设计

### 合并策略：crab-auth 代码搬入 crab-cloud

crab-auth 的模块以子模块形式迁入 crab-cloud，保持清晰边界：

```
crab-cloud/src/
├── api/
│   ├── mod.rs              # 路由 (新增 auth 路由组)
│   ├── health.rs
│   ├── register.rs
│   ├── sync.rs
│   ├── tenant.rs
│   ├── stripe_webhook.rs
│   ├── update.rs
│   └── pki/                # ← 从 crab-auth 迁入
│       ├── mod.rs           # 子路由
│       ├── activate.rs      # POST /api/server/activate
│       ├── activate_client.rs
│       ├── deactivate.rs
│       ├── deactivate_client.rs
│       ├── subscription.rs  # POST /api/tenant/subscription
│       ├── binding.rs       # POST /api/binding/refresh
│       ├── verify.rs
│       ├── p12.rs           # POST /api/p12/upload
│       └── pki.rs           # GET /pki/root_ca
├── db/
│   ├── mod.rs
│   ├── tenants.rs          # 已有 (crab-cloud)
│   ├── subscriptions.rs    # 已有 (crab-cloud)
│   ├── sync_store.rs
│   ├── activations.rs      # ← 从 crab-auth 迁入
│   ├── client_connections.rs # ← 从 crab-auth 迁入
│   └── p12.rs              # ← 从 crab-auth 迁入
├── state.rs                # AppState 合并 (见下)
└── ...
```

### AppState 合并

crab-auth 的字段合并到 crab-cloud AppState：

```rust
pub struct AppState {
    // --- 已有 (crab-cloud) ---
    pub pool: PgPool,
    pub ses: SesClient,
    pub s3: S3Client,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub ses_from_email: String,
    pub registration_success_url: String,
    pub registration_cancel_url: String,
    pub update_s3_bucket: String,
    pub update_download_base_url: String,
    pub jwt_secret: String,
    pub quota_cache: QuotaCache,
    pub rate_limiter: RateLimiter,
    pub root_ca_pem: String,

    // --- 新增 (从 crab-auth) ---
    pub ca_store: CaStore,     // 替换简化版，用 crab-auth 的完整版
    pub sm: SmClient,          // Secrets Manager (P12 密码存储)
    pub p12_s3_bucket: String, // P12 证书 S3 桶
    pub kms_key_id: Option<String>, // KMS Key ID
}
```

### CaStore 统一

用 crab-auth 的完整 CaStore 替换 crab-cloud 的精简版：
- 支持 Root CA / Tenant CA 的创建和读取（含私钥）
- crab-cloud 原有的 `load_tenant_ca_cert()` 改为调用 `load_tenant_ca()`
- Root CA PEM 从 CaStore 获取，不再依赖文件

### 数据库迁移

crab-auth 的 3 个迁移文件合并到 crab-cloud/migrations/：
- `0007_activations.up.sql` ← crab-auth/0001
- `0008_p12_certificates.up.sql` ← crab-auth/0002
- `0009_client_connections.up.sql` ← crab-auth/0003

### 路由

所有 crab-auth 端点保持原有路径不变，挂到 crab-cloud 的 public_router：

```
# 新增 PKI 路由 (从 crab-auth 迁入)
POST /api/server/activate
POST /api/client/activate
POST /api/server/deactivate
POST /api/client/deactivate
POST /api/tenant/subscription    # 注意: 与 crab-cloud 现有 tenant 路由同级
POST /api/tenant/verify
POST /api/binding/refresh
POST /api/p12/upload             # 5MB body limit
GET  /pki/root_ca
```

### DNS 变更

`auth.redcoral.app` → 指向 EC2 (51.92.72.162)，由 Caddy 处理 HTTPS

Caddyfile 新增：
```
auth.redcoral.app {
    reverse_proxy crab-cloud:8080
}
```

### 客户端兼容

edge-server 使用 `shared::DEFAULT_AUTH_SERVER_URL = "https://auth.redcoral.app"`。
合并后 URL 不变，端点路径不变，**零客户端改动**。

### 删除清单

合并完成后：
- 删除 `crab-auth/` crate 整个目录
- 从 `Cargo.toml` workspace members 移除 `crab-auth`
- 删除 `crab-auth/Dockerfile.lambda`
- 清理 AWS 资源：Lambda 函数、API Gateway 自定义域、VPC Endpoints (SM + STS)、Lambda 安全组

### 新增依赖

crab-cloud 需新增：
- `aws-sdk-kms` (P12 SSE-KMS 加密)
- ~~`lambda_http`~~ 不需要了

### 费用影响

| 项目 | 合并前 | 合并后 |
|------|--------|--------|
| VPC Endpoints (SM + STS) | $14.4/月 | $0 |
| Lambda | $0 | $0 (删除) |
| API Gateway | $0 | $0 (删除) |
| EC2 | $6.1/月 | $6.1/月 (不变) |
| **净省** | | **$14.4/月** |
