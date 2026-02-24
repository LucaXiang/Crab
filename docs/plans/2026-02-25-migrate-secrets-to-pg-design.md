# 迁移 Tenant CA + P12 从 Secrets Manager 到 PostgreSQL

**日期**: 2026-02-25
**状态**: implemented
**动机**: AWS Secrets Manager 按 secret 数量计费 ($0.40/secret/月)，每新增租户 +2 secret。迁移到 PG 后费用固定 $2/月。

## 变更摘要

- Tenant CA (cert_pem + key_pem) 存入 `tenants` 表
- P12 数据 (base64) + 密码存入 `p12_certificates` 表，移除 `secret_name` 列
- Root CA 保留在 Secrets Manager
- `AppState.sm` 字段移除，`CaStore` 内部保留 `SmClient` 仅用于 Root CA
- DashMap + OnceCell 内存缓存不变

## 费用对比

| 场景 | 迁移前 | 迁移后 |
|---|---|---|
| 12 租户 | $11.2/月 | $2.0/月 |
| 10000 租户 | $8001.2/月 | $2.0/月 |
