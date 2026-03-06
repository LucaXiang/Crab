# 全栈 Tracing & ErrorCode 审计整改方案

**日期**: 2026-03-06
**范围**: edge-server + crab-cloud + red_coral/src-tauri + shared

## 目标

1. **精简噪声** — 删除冗余 tracing，让日志对排查问题有帮助
2. **暴露静默错误** — 不该吞掉的错误必须 warn/error
3. **保持错误精度** — 有对应 ErrorCode 的错误不降级为通用 fallback

---

## Phase 1: Tracing 噪声精简

### 1.1 Edge-server 删除/降级 (~35 条)

**删除（纯噪声）**:
- `services/cert.rs` — 中间步骤日志 (L78,97,282,327,333)，保留最终 L341
- `services/activation.rs` — "Self-check passed!" (L313,366), "verified successfully" (L603,832)
- `message/tcp_server.rs` — per-client lifecycle logs (15+ 处)
- `api/kitchen_orders/handler.rs:284` — "Label reprinted successfully"
- `core/tasks.rs:226` — "Task completed"
- `orders/storage.rs:213` — "Archive completed..."
- API handlers — "updated successfully" 类确认日志 (products, categories 等)

**聚合**:
- `services/activation.rs:611-692` — 删除 "Running sync..."，保留最终状态
- `services/cert.rs:272-341` — 6 条自检日志聚合为 1 条最终 info

**降级 info → debug**:
- `daily_reports.rs` — "already exists, skipping" 幂等检查

### 1.2 Cloud 补关键路径 tracing

- `db/sync_store.rs` — sync 失败路径加 warn (entity_id, resource_type)
- `api/stripe_webhook.rs` — 支付事件接收加 info
- `api/ws.rs` / `console_ws.rs` — WS 连接生命周期加 debug
- PKI 签发/吊销操作加 info

### 1.3 Tauri 轻量补充

- `commands/data.rs`, `sync.rs`, `statistics.rs` — error path 加 warn
- 不做全面 instrument

---

## Phase 2: 静默错误修复

### 2.1 严重 — 必须修 (~10 处)

**edge-server**:
1. `cloud/worker.rs:661` — catalog_changelog 同步标记 `let _ =` → `if let Err(e)` + error!
2. `cloud/worker.rs:719` — chain_entry 序列化 `if let Ok` → warn! 跳过的 entry
3. `api/data_transfer/handler.rs:681-805` — broadcast_catalog_sync 7 个资源查询 `if let Ok` → warn!
4. `services/catalog_service.rs:205` — attribute_bindings `if let Ok` → error! (影响价格计算)

**crab-cloud**:
5. `stripe_webhook.rs` L249,286,333,387,395,399 — 6 处 `let _ =` 租户状态更新 → `if let Err(e)` + error!
6. `api/tenant/auth.rs:133` — 验证码 upsert `let _ =` → 先检查 upsert 结果再发邮件

### 2.2 中等 — 应加 warn (~17 处)

**edge-server**:
- `cloud/worker.rs:978,1001,1008` — rollback 失败加 warn
- `cloud/ops/attribute.rs:269,272,310,316` — 缓存刷新失败加 warn
- `db/repository/label_template.rs:142,242,250,260` — 图片引用同步加 warn
- `services/catalog_service.rs:614` — 商品图片引用同步加 warn
- `api/kitchen_orders/handler.rs:362,418` — 反序列化失败加 warn (含 event_id)
- `archiving/worker.rs:590` — 班次序列化失败加 warn
- `api/employees/handler.rs:147` — 审计信息查询失败加 warn

**crab-cloud**:
- `api/tenant/command.rs:86` — 命令完成记录加 warn
- `auth/quota.rs:165` — DB 失败误判加 warn
- `api/ws.rs:127` — pending_ops fetch 失败加 warn
- `api/ws.rs:459` — presigned_url 失败加 warn
- `api/image.rs:120` — S3 列举失败加 warn

**tauri**:
- `order_es.rs:177` — 序列化失败加 warn (当前 .ok() 丢弃)
- `order_es.rs:197` — 反序列化失败不构造假成功响应，改为 warn + 返回错误
- `lifecycle.rs:298,590,723` — session 清理失败加 warn

---

## Phase 3: ErrorCode 精度修复

### 3.1 Edge: CommandErrorCode::InternalError 分化 (18 处)

`orders/manager/mod.rs:498-599` 中 InternalError 应分化为：
- StorageCorrupted — redb 读写失败
- SystemBusy — 锁获取失败
- StorageFull — 存储满
- 已有的 StampActivityNotFound 等

### 3.2 Edge: internal(e.to_string()) 精确化 (24 处)

`api/data_transfer/handler.rs:58-208` — ZIP/JSON/文件操作用具体 ErrorCode:
- 新增: `ImportInvalidFormat`, `ImportFileTooLarge`, `ExportFailed`

`message/tcp_server.rs:40-151` — TLS 握手用具体 ErrorCode

### 3.3 Cloud: sqlx 错误分类

`crab-cloud/src/error.rs:42-52` — `ServiceError::from(sqlx::Error)` 改为:
```rust
match e {
    sqlx::Error::RowNotFound => ErrorCode::NotFound,
    sqlx::Error::Database(ref db_err) => match db_err.code().as_deref() {
        Some("23505") => ErrorCode::AlreadyExists,
        Some("23503") => ErrorCode::ValidationFailed,
        _ => ErrorCode::DatabaseError,
    },
    _ => ErrorCode::InternalError,
}
```

### 3.4 Cloud: 验证字符串 → ErrorCode

新增 ErrorCode:
- `PriceRuleValueOutOfRange` — 价格规则百分比/金额范围
- `CatalogImportInvalidFormat` — ZIP 格式错误
- `CatalogImportMissingData` — JSON 缺失
- `PasswordHashingFailed` — 密码哈希失败

### 3.5 Shared: 死代码清理

删除 54 个从未使用的 ErrorCode variant:
- 文件上传组 (9): FileTooLarge, UnsupportedFileFormat...
- 订单支付组 (10): OrderAlreadyPaid, PaymentFailed...
- 认证组 (5): TokenInvalid, SessionExpired, AccountLocked...
- 产品组 (7): ProductOutOfStock, SpecNotFound...
- 系统组 (11): NetworkError, TimeoutError...
- 员工/角色/租户/桌台组 (12)

**注意**: 删除前需确认前端 i18n 中无对应翻译 key 被引用。

### 3.6 前端 i18n 同步

新增 ErrorCode 必须同步:
- `red_coral/src/infrastructure/i18n/locales/zh-CN.json`
- `red_coral/src/infrastructure/i18n/locales/es-ES.json`
- `crab-console/` 对应翻译文件

---

## 执行策略

按 Phase 分支，每 Phase 独立 PR：

| Phase | 范围 | 估计变更文件 | 风险 |
|-------|------|------------|------|
| 1 | Tracing 精简 | ~20 文件 | 极低 (纯日志) |
| 2 | 静默错误修复 | ~15 文件 | 低 (加日志/错误处理) |
| 3 | ErrorCode 精度 | ~30 文件 (含 shared + i18n) | 中 (改错误类型影响 API 响应) |

建议 Phase 1+2 合并执行（都是日志/错误处理），Phase 3 单独执行（涉及 API 响应变更）。
