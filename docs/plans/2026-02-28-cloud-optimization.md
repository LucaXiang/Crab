# crab-cloud 优化方案

> 基于 2026-02-28 架构审计，覆盖连接池、中间件、同步管线、数据库 schema 四个维度。
> 当前架构舒适区 ~30 租户 / ~50 门店，目标提升至 100+ 租户 / 200+ 门店。

## 一、代码级优化（改动小，收益大）

### 1.1 PG 连接池扩容

**文件**: `crab-cloud/src/state.rs:82-85`

**现状**: `max_connections=50`, `min_connections=2`, `acquire_timeout=5s`
**问题**: 所有 HTTP API + Edge WS sync + Console 查询共享 50 个连接。一次 SyncBatch 处理期间长期占住连接（逐条 upsert）。

**修复**:
```rust
.max_connections(100)  // 50 → 100
.min_connections(5)    // 2 → 5
```

同时需要确保 PG 端 `max_connections` >= 150（留给其他工具 50）。
在 docker-compose.yml 中加 PG 配置:
```yaml
command: postgres -c max_connections=200
```

**风险**: 无。连接池只是上限，不会预创建。

---

### 1.2 QuotaCache RwLock 读锁泄漏（Bug）

**文件**: `crab-cloud/src/auth/quota.rs:60-69`

**现状**: 读锁在 cache hit 路径上被持有，跨越了 `next.run(request).await`（整个 handler 执行期间）。
任何并发的 cache miss 写入都被阻塞。

```rust
// 当前代码 — 读锁跨越整个 handler
{
    let entries = state.quota_cache.entries.read().await;
    if let Some(entry) = entries.get(&cache_key)
        && entry.expires_at > Instant::now()
    {
        if let Some(code) = entry.error {
            return Err(AppError::new(code).into_response());
        }
        return Ok(next.run(request).await);  // ← 读锁还在！
    }
}
```

**修复**: 先提取结果，再释放锁:
```rust
let cached_result = {
    let entries = state.quota_cache.entries.read().await;
    entries.get(&cache_key)
        .filter(|e| e.expires_at > Instant::now())
        .map(|e| e.error)
};
// 锁已释放
if let Some(maybe_err) = cached_result {
    if let Some(code) = maybe_err {
        return Err(AppError::new(code).into_response());
    }
    return Ok(next.run(request).await);
}
```

或更进一步：将 `Arc<RwLock<HashMap>>` 替换为 `DashMap`，彻底消除全局写锁。

---

### 1.3 CaStore 不必要的 AES 解密

**文件**: `crab-cloud/src/state.rs:334-356` (`load_tenant_ca_cert`)

**现状**: mTLS 验证只需要 tenant CA 的公钥证书（cert_pem），但 cache miss 时同时查询并解密了私钥（ca_key_encrypted）。AES-256-GCM 解密是不必要的 CPU 开销。

**修复**: `load_tenant_ca_cert()` 应该只查询 `ca_cert_pem` 列，不查 `ca_key_encrypted`，不调用 `decrypt_string`。同时维护一个独立的 cert-only 缓存（或复用现有 `tenant_ca_cache` 但允许 key 为空）。

---

### 1.4 edge_auth 每次请求查 DB 验证 activation

**文件**: `crab-cloud/src/auth/edge_auth.rs:81-90`

**现状**: 每次 edge sync/WS 请求都 `SELECT ... FROM activations WHERE entity_id = $1`。对于 WebSocket 只在 upgrade 时查一次（可接受），但如果未来有 HTTP sync 路径则每批都查。

**修复**: 在 `AppState` 中加一个 `DashMap<String, (bool, Instant)>` 的 activation 缓存，TTL 5 分钟。WebSocket 连接是长连接，所以当前影响不大，优先级低。

---

## 二、同步管线优化

### 2.1 SyncBatch 写入放大分析

当前每种资源类型的 PG 查询数:

| 资源类型 | 查询数 (min/max) | 有事务 | 有 stale-skip |
|----------|-----------------|--------|---------------|
| Tag, Employee, Zone, PriceRule, Binding | 1/1 | 否 | 是 |
| Shift | 1/1 | 否 | 否 |
| StoreInfo | 1/1 | 否 | 条件 |
| ArchivedOrder, CreditNote, Invoice | 1/1 | 否 | 是 |
| Attribute | 1/3 | 是 | 是 |
| LabelTemplate | 1/3 | 是 | 是 |
| Product | 1/5 | 是 | 是 |
| Category | 1/5 | 是 | 是 |
| **DailyReport** | **4/7** | 是 | **缺失** |

典型全量同步（100 products + 20 categories + 10 attributes + 5 daily reports）= **~667 条 SQL**。

### 2.2 DailyReport 缺少 stale-skip（Bug + 性能）

**文件**: `crab-cloud/src/db/store/daily_report.rs`

**现状**: `upsert_daily_report_from_sync` 的 `ON CONFLICT DO UPDATE` 没有 `WHERE updated_at <= EXCLUDED.updated_at` 守卫。每次同步都无条件删除并重建 3 个 breakdown 子表（tax / payment / shift），即使 cloud 已有更新版本。

**修复**: 加 stale-skip 守卫，与 Product/Category 保持一致：
```sql
ON CONFLICT (store_id, source_id) DO UPDATE SET ...
WHERE store_daily_reports.updated_at <= EXCLUDED.updated_at
RETURNING id
```
RETURNING NULL 时直接返回，跳过子表操作。

### 2.3 批量化方向（中期）

当前逐条 `upsert_resource()` 的模式：
```rust
for item in items {
    upsert_resource(&state.pool, &identity, item).await?;
}
```

**可优化方向**:
1. **同类型资源合并**: 将 batch 内同类型资源收集后一次性 UNNEST 批量 INSERT
2. **整批包在一个事务内**: 减少事务提交开销
3. **ArchivedOrder/CreditNote/Invoice 批量 INSERT**: 这些是 append-only 的单查询资源，可以用 UNNEST 一次插入多条

**评估**: 对于增量同步（每批 1-10 条），当前模式足够。对于初始全量同步（几百条），批量化收益显著。建议在观察到实际性能问题后再实施。

---

## 三、数据库 Schema 优化

### P0：必须修复

#### 3.1 `get_overview` JSONB 展开缺覆盖索引

**文件**: `crab-cloud/src/db/tenant_queries.rs:757-826`

**现状**: Overview 请求触发 9 个并发查询，其中 5 个对 `detail` JSONB 列做 `CROSS JOIN jsonb_array_elements()`。tenant-wide 查询（`store_id IS NULL`）时现有索引退化。

**修复** — 加覆盖索引:
```sql
CREATE INDEX idx_archived_orders_overview
    ON store_archived_orders (tenant_id, end_time)
    INCLUDE (store_id, status, total, tax, guest_count,
             discount_amount, start_time, void_type, loss_amount)
    WHERE end_time IS NOT NULL;
```
让聚合查询（SUM/COUNT/AVG）走 Index-Only Scan，避免 heap fetch。

#### 3.2 `get_red_flags` 改用 end_time 过滤

**文件**: `crab-cloud/src/db/tenant_queries.rs:1010-1035`

**现状**: 按 JSONB 内部的 `e->>'timestamp'` 过滤，必须全行读取 detail 后展开。

**修复**: 改为先按 `end_time` 范围过滤行集（利用索引），再展开 events:
```sql
WHERE o.store_id = $1 AND o.tenant_id = $2
    AND o.end_time >= $3 AND o.end_time < $4  -- 先缩减行集
    AND o.detail IS NOT NULL
    AND e->>'event_type' IN (...)
```

#### 3.3 audit_logs 加清理策略

**修复**:
- 在 `main.rs` 的 cleanup 循环中加 audit_logs 清理（保留 90 天）
- 补充索引: `CREATE INDEX idx_audit_logs_tenant_action ON audit_logs (tenant_id, action, created_at DESC);`

### P1：近期修复

#### 3.4 refresh_tokens 撤销行清理

```sql
-- 合并为条件复合索引
CREATE INDEX idx_refresh_tokens_active
    ON refresh_tokens (tenant_id, expires_at DESC)
    WHERE NOT revoked;
```
加定时清理: 删除 30 天前已撤销的 token。

#### 3.5 processed_webhook_events 加 TTL

Stripe 幂等窗口 48h，保留 7 天后清理。

#### 3.6 store_attribute_bindings 索引补 store_id

```sql
-- 原: (owner_type, owner_source_id) — 多租户环境下不唯一
-- 改: (store_id, owner_type, owner_source_id)
```

#### 3.7 daily_reports 查询去掉冗余 JOIN stores

`verify_store_ownership()` 已在上游验证，查询中不需要再 JOIN stores。

### P2：计划修复

- credit_notes 时间范围索引: `(tenant_id, store_id, created_at)`
- store_invoices AEAT pending 索引加 store_id
- 高价值表 CASCADE 改 RESTRICT（archived_orders, credit_notes, invoices）
- store_pending_ops 加清理

### P3：未来

- store_archived_orders 按 end_time 月份分区（年订单 10 万+ 时）

---

## 四、运维层优化

### 4.1 PG 备份

```bash
# 加到 crontab (EC2 上)
0 3 * * * docker exec crab-postgres pg_dump -U crab crab | gzip > /opt/crab/backups/crab-$(date +\%Y\%m\%d).sql.gz
# 保留 30 天
find /opt/crab/backups -name "*.sql.gz" -mtime +30 -delete
```

### 4.2 Docker 资源限制

```yaml
# docker-compose.yml
crab-cloud:
  deploy:
    resources:
      limits:
        memory: 1G
        cpus: '2'

postgres:
  deploy:
    resources:
      limits:
        memory: 2G
        cpus: '2'
  command: postgres -c max_connections=200 -c shared_buffers=512MB -c work_mem=8MB
```

### 4.3 定期清理任务（新增到 main.rs）

在现有 cleanup 循环中补充:
```rust
// 每小时运行一次
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        // audit_logs: 90 天
        // refresh_tokens: 30 天已撤销
        // processed_webhook_events: 7 天
        // store_commands: 90 天已完成
        // store_pending_ops: 30 天
    }
});
```

### 4.4 分离 dev 环境

当前 prod 和 dev 共享 EC2。建议:
- 短期: 给 dev 容器设资源上限，避免抢占 prod
- 中期: dev 迁移到另一台 EC2 或本地 Docker

---

## 五、实施优先级

| 阶段 | 内容 | 工作量 | 预期效果 |
|------|------|--------|---------|
| **立即** | 1.1 连接池 + 1.2 QuotaCache bug + 3.2 red_flags SQL | 1h | 并发容量 2x，消除锁竞争 |
| **本周** | 2.2 DailyReport stale-skip + 3.1 覆盖索引 + 3.3 audit 清理 | 2-3h | 写入放大减少，查询性能提升 |
| **下周** | 1.3 CaStore 优化 + 3.4-3.7 索引修复 + 4.1 备份 | 3-4h | 安全性 + 查询效率 |
| **中期** | 2.3 批量化 + 4.2-4.4 运维 | 1-2d | 大规模同步性能 |

---

## 总结

当前架构的主要瓶颈不在 CPU/内存，而在:
1. **PG 连接池过小**（50，改 100 立竿见影）
2. **QuotaCache 读锁泄漏**（bug，影响并发 edge 同步）
3. **Overview 查询 JSONB 全表扫描**（加覆盖索引）
4. **无清理策略的表**（audit_logs, refresh_tokens 等逐步膨胀）

修复以上问题后，同一台 EC2 可舒适支撑 100+ 租户 / 200+ 门店。
