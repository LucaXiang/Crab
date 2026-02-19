# CLAUDE.md

## Crab Cloud

云端统一服务 — 租户管理、PKI/认证、设备激活、订阅校验、Stripe 集成、边缘数据同步。

## 命令

```bash
cargo check -p crab-cloud
cargo test -p crab-cloud --lib
```

## 模块结构

```
src/
├── main.rs          # Axum 长驻服务入口 (HTTP + mTLS 双端口)
├── config.rs        # DATABASE_URL, MTLS_PORT, ROOT_CA, P12 配置
├── state.rs         # AppState { pool, ca_store, sm, ... } + CaStore (PKI 全功能)
├── auth/
│   ├── mod.rs
│   ├── edge_auth.rs # mTLS + SignedBinding 验证 → EdgeIdentity
│   ├── tenant_auth.rs # JWT 租户认证
│   ├── rate_limit.rs  # 速率限制
│   └── quota.rs       # 配额校验
├── api/
│   ├── mod.rs         # public_router + edge_router
│   ├── health.rs      # GET /health
│   ├── register.rs    # POST /api/register, verify-email, resend-code
│   ├── sync.rs        # POST /api/edge/sync (mTLS)
│   ├── tenant.rs      # 租户管理 API (profile, billing, stores, commands)
│   ├── update.rs      # 应用更新检查
│   ├── stripe_webhook.rs # Stripe webhook
│   └── pki/           # PKI 路由 (原 crab-auth)
│       ├── mod.rs
│       ├── activate.rs         # POST /api/server/activate
│       ├── activate_client.rs  # POST /api/client/activate
│       ├── deactivate.rs       # POST /api/server/deactivate
│       ├── deactivate_client.rs # POST /api/client/deactivate
│       ├── verify.rs           # POST /api/tenant/verify
│       ├── subscription.rs     # POST /api/tenant/subscription
│       ├── binding.rs          # POST /api/binding/refresh
│       ├── refresh.rs          # POST /api/tenant/refresh (token rotation)
│       ├── p12.rs              # POST /api/p12/upload
│       └── root_ca.rs          # GET /pki/root_ca
└── db/
    ├── mod.rs
    ├── tenants.rs         # 租户 CRUD + 认证
    ├── subscriptions.rs   # 订阅管理
    ├── activations.rs     # 服务器激活记录
    ├── client_connections.rs # 客户端连接记录
    ├── refresh_tokens.rs  # Refresh token 存储 + 轮转
    ├── p12.rs             # P12 证书元数据
    ├── sync_store.rs      # 边缘同步数据镜像
    ├── audit.rs           # 审计日志
    ├── commands.rs        # 远程命令
    ├── email_verifications.rs # 邮箱验证
    └── tenant_queries.rs  # 租户查询聚合
```

## 认证模型

1. **JWT + Refresh Token**: 租户 verify → JWT access token (1h) + refresh token (30 days, rotate-on-use)
   - activate/deactivate 等操作使用 JWT token，不再传 username/password
   - refresh token 存储在 PostgreSQL，每次使用后轮转（旧 token 废弃，签发新 token）
2. **mTLS**: Entity Cert (Tenant CA 签发) → Root CA 验证证书链（激活后的边缘同步）
3. **SignedBinding**: HTTP header `X-Signed-Binding` → Tenant CA cert 验签（订阅检查、binding 刷新）

## PKI 层级

Root CA → Tenant CA → Entity Cert (Server / Client)
- 存储: AWS Secrets Manager
- P12: S3 + KMS 加密

## 数据库

PostgreSQL — 统一管理所有表 (tenants, subscriptions, activations, client_connections, refresh_tokens, p12_certificates, cloud_*)

## 部署

EC2 + Docker Compose + Caddy (自动 HTTPS)。详见 [DEPLOY.md](DEPLOY.md)。

部署目录: `/opt/crab/` (EC2 上)
配置文件: `deploy/ec2/` (docker-compose.yml, Caddyfile, setup.sh)
构建脚本: `deploy/build-cloud.sh` (Docker build + ECR push)

## 响应语言

使用中文回答。
