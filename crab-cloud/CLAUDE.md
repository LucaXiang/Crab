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
├── state.rs         # AppState { pool, ca_store, sm, edges } + CaStore (PKI 全功能)
├── auth/
│   ├── mod.rs
│   ├── edge_auth.rs   # mTLS + SignedBinding 验证 → EdgeIdentity
│   ├── tenant_auth.rs # JWT 租户认证
│   ├── rate_limit.rs  # 速率限制
│   └── quota.rs       # 配额校验
├── api/
│   ├── mod.rs         # public_router + edge_router
│   ├── health.rs      # GET /health
│   ├── register.rs    # POST /api/register, verify-email, resend-code
│   ├── sync.rs        # POST /api/edge/sync (mTLS)
│   ├── update.rs      # 应用更新检查
│   ├── image.rs       # 图片代理
│   ├── console_ws.rs  # Console WebSocket (实时在线状态)
│   ├── ws.rs          # Edge WebSocket (mTLS, StoreOp 推送)
│   ├── stripe_webhook.rs # Stripe webhook
│   ├── store/         # 门店资源 Console CRUD (cloud→edge 双向)
│   │   ├── mod.rs         # store_router + push_to_edge_if_online
│   │   ├── product.rs     # 商品 CRUD
│   │   ├── category.rs    # 分类 CRUD
│   │   ├── attribute.rs   # 属性 CRUD
│   │   ├── tag.rs         # 标签 CRUD
│   │   ├── employee.rs    # 员工 CRUD
│   │   ├── zone.rs        # 区域 CRUD
│   │   ├── dining_table.rs # 餐桌 CRUD
│   │   ├── price_rule.rs  # 价格规则 CRUD
│   │   └── label_template.rs # 标签模板 CRUD
│   ├── tenant/        # 租户管理 API
│   │   ├── mod.rs         # tenant_router
│   │   ├── analytics.rs   # stats, overview, red-flags
│   │   ├── store.rs       # list/update stores
│   │   ├── order.rs       # 归档订单查询
│   │   └── command.rs     # 远程命令
│   └── pki/           # PKI 路由
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
    ├── store/             # 门店资源 normalized 表操作
    │   ├── mod.rs             # snowflake_id() + increment_store_version()
    │   ├── product.rs         # 商品 (含 specs 子表)
    │   ├── category.rs        # 分类 (含 tag 关联)
    │   ├── attribute.rs       # 属性 (含 options/bindings 子表)
    │   ├── tag.rs             # 标签
    │   ├── employee.rs        # 员工
    │   ├── zone.rs            # 区域
    │   ├── dining_table.rs    # 餐桌
    │   ├── price_rule.rs      # 价格规则
    │   ├── label_template.rs  # 标签模板
    │   ├── daily_report.rs    # 日报 (含 tax/payment breakdown 子表)
    │   ├── shift.rs           # 班次
    │   └── store_info.rs      # 门店信息 (singleton per edge)
    ├── sync_store.rs      # 边缘同步数据写入 (normalized, 无 JSONB)
    ├── audit.rs           # 审计日志
    ├── commands.rs        # 远程命令
    ├── email_verifications.rs # 邮箱验证
    └── tenant_queries.rs  # 租户查询聚合 (overview, red-flags, daily reports)
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

PostgreSQL — 所有门店资源使用 normalized 表（无 JSONB），表名前缀 `store_*`。

**ID 生成**: Console CRUD 创建资源时使用 `snowflake_id()` 生成 `source_id`（41-bit 时间戳 + 22-bit 随机），与 edge SQLite rowid 互不冲突。

**双向同步**:
- Edge → Cloud: `sync_store.rs` 接收 edge 同步数据，写入 normalized 表
- Cloud → Edge: `api/store/` Console CRUD 操作后通过 WebSocket 推送 `StoreOp` 到 edge

**主要表组**:
- 平台: tenants, subscriptions, activations, client_connections, refresh_tokens, p12_certificates
- 门店资源: store_products, store_categories, store_tags, store_attributes, store_employees, store_zones, store_dining_tables, store_price_rules, store_label_templates
- 门店数据: store_daily_reports, store_shifts, store_info
- 子表: store_product_specs, store_attribute_options, store_attribute_bindings, store_category_tag, store_daily_report_tax_breakdown, store_daily_report_payment_breakdown

## 部署

EC2 + Docker Compose + Caddy (自动 HTTPS)。详见 [DEPLOY.md](DEPLOY.md)。

部署目录: `/opt/crab/` (EC2 上)
配置文件: `deploy/ec2/` (docker-compose.yml, Caddyfile, setup.sh)
构建脚本: `deploy/build-cloud.sh` (Docker build + ECR push)

## 响应语言

使用中文回答。
