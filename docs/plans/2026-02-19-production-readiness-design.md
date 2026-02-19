# Production Readiness Design

## Overview

Crab POS 系统生产就绪评估。Edge-server 运行在局域网内，不对公网暴露。

**基线**: 103,926 行 Rust / 1,034 测试全通过 / CI/CD 完整 / mTLS 3级PKI / Event Sourcing

**原则**: 不过度重构。只修真正有风险的问题。

## 评估结论

系统整体架构优秀，安全基础扎实。需要做的事很少：

## 需要做的

### 1. PostgreSQL 备份自动化 (P0 — 数据安全)

**问题**: `pgdata` Docker volume 未备份，EC2 故障 = 租户数据全丢。

```bash
# /opt/crab/backup.sh
#!/bin/bash
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
docker exec crab-postgres pg_dump -U crab crab | gzip > /opt/crab/backups/crab-${TIMESTAMP}.sql.gz
aws s3 cp /opt/crab/backups/crab-${TIMESTAMP}.sql.gz s3://crab-backups/pg/
find /opt/crab/backups -name "*.sql.gz" -mtime +7 -delete

# crontab: 0 2 * * * /opt/crab/backup.sh
```

### 2. Docker 日志轮转 (P1 — 磁盘安全)

**问题**: docker-compose.yml 无 logging 配置，容器日志无限增长，撑满 EC2 磁盘。

每个 service 添加 3 行：

```yaml
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

### 3. SQLite busy_timeout (P1 — 写入可靠性)

**问题**: 默认 busy_timeout=0，写冲突立即失败。高峰期订单 + 归档 + API 同时写会 SQLITE_BUSY。

`edge-server/src/db/mod.rs` 连接池创建后加一行：

```rust
sqlx::query("PRAGMA busy_timeout = 5000;").execute(&pool).await?;
```

### 4. Cloud Health 加 PG 检查 (P1 — 故障发现)

`crab-cloud/src/api/health.rs` 仅返回 git hash，不检查 PG 连接。加个 `SELECT 1` 检查。

## 不需要做的（以及为什么）

| 项目 | 原因 |
|------|------|
| CORS 收紧 | Edge 在局域网，Server 模式走 oneshot 不经 HTTP，Client 模式走 mTLS TCP |
| Edge 请求体限制 | 局域网内无外部攻击面 |
| Edge Rate Limiting | 局域网 + mTLS 双向认证，暴力破解不现实 |
| /metrics + Prometheus | 开发阶段，有问题看日志就够 |
| 连接池调优 | 没有性能问题不改 |
| Runbook | 自己开发运维，CLAUDE.md 里已有部署流程 |
| 日志聚合 CloudWatch | 直接 SSH 看日志就行 |
| 负载测试 | 单店 POS 并发有限，实际使用就是测试 |
| 分布式追踪 | 过度 |
| Multi-AZ | MVP 阶段成本过高 |
| 前端测试补全 | TS 类型检查 + 手工测试足够 |
| Sentry | tracing 日志足够 |

## 正面发现

这些已经做得很好，不需要动：

- Event Sourcing + SHA256 哈希链验证
- mTLS 3级 PKI + 硬件绑定
- 1,034 测试全通过，Rust 代码零 TODO
- Background tasks `catch_unwind` 保护
- Dead Letter Queue 隔离失败归档
- 自动订阅检查 + 指数退避重试
- CI/CD (clippy 零警告 + TS 类型检查 + 自动部署)
- 审计日志系统 (6 模块异步写入)
- Argon2 密码 + JWT rotate-on-use
- Cloud Rate Limiting (login 5/min)
