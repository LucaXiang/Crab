# CLAUDE.md

## Crab Cloud

云端租户管理中心 — 接收 edge-server 同步数据，未来提供远程访问和 Stripe 集成。

## 命令

```bash
cargo check -p crab-cloud
cargo test -p crab-cloud --lib
```

## 模块结构

```
src/
├── main.rs          # Axum 长驻服务入口
├── config.rs        # DATABASE_URL, MTLS_PORT, ROOT_CA 路径
├── state.rs         # AppState { pool, ca_store }
├── auth/
│   ├── mod.rs
│   └── edge_auth.rs # mTLS + SignedBinding 验证 → EdgeIdentity
├── api/
│   ├── mod.rs       # Router
│   ├── sync.rs      # POST /api/edge/sync
│   └── health.rs    # GET /health
└── db/
    ├── mod.rs
    └── sync_store.rs  # 数据镜像 upsert 操作
```

## 认证模型

1. **mTLS**: Entity Cert (Tenant CA 签发) → Root CA 验证证书链
2. **SignedBinding**: HTTP header `X-Signed-Binding` → Tenant CA cert 验签

## 数据库

与 crab-auth 共享 PostgreSQL 实例。crab-cloud 拥有 `cloud_*` 表，读取 crab-auth 的 `tenants`/`subscriptions` 表。

## 响应语言

使用中文回答。
