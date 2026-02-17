# crab-cloud 全面完善设计

**日期**: 2026-02-17
**状态**: 已批准

## 背景

crab-cloud 约 60% 功能完整。已实现：注册+Stripe 支付、edge-server 数据同步、mTLS 双端口、命令通道、租户管理 API。

需要全面提升到生产就绪：加固 + 账户管理 + Stripe 补全 + 审计日志。

## 现状评估

### 已完成
- 租户注册 + Stripe 支付流程
- Edge-server 数据同步（7 种资源类型）
- mTLS 双端口（HTTP :8080 + mTLS :8443）
- Cloud → Edge 命令通道
- 租户管理 API（8 个端点）
- 应用更新分发（S3 + semver）

### 缺失
- 安全加固（webhook 幂等性、事务边界、消除 panic）
- 账户管理（密码重置、邮箱修改）
- 邮件通知（订阅状态变更）
- Stripe 补全（退款、续费、Customer Portal）
- 审计日志

---

## Part 1: 生产加固

### 1.1 Webhook 幂等性

新建 migration：

```sql
CREATE TABLE IF NOT EXISTS processed_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_at BIGINT NOT NULL
);
```

`stripe_webhook.rs` 入口处查 `processed_webhook_events`，已存在则 200 跳过。处理完后 INSERT。webhook 处理包在 PG 事务中。

### 1.2 注册事务边界

`register.rs` 的 `register()` 将 tenant 创建 + email_verification 写入包在 `pool.begin()` 事务中。邮件发送在事务提交后执行（失败不影响注册，用户可 resend）。

### 1.3 JWT_SECRET 强制

`config.rs`：production 环境 `JWT_SECRET` 未设置时 panic。development 保留默认值。

### 1.4 消除 unwrap/panic

- `commands.rs` — `parse::<i64>().unwrap_or(0)` → 返回 error 跳过
- `stripe/mod.rs` — 所有 `.unwrap()` → `Result` propagation + 日志
- 其余已有 proper error handling，保持

### 1.5 应用层 Rate Limiting

- `POST /api/tenant/login` — 5 次/分钟/IP
- `POST /api/register` — 3 次/分钟/IP
- 用内存 HashMap<IP, (count, window_start)> + 过期清理
- 其他端点依赖 WAF 层全局限流

### 文件清单

| 文件 | 操作 |
|------|------|
| 新 migration | 新建 `processed_webhook_events` 表 |
| `api/stripe_webhook.rs` | 幂等性检查 + 事务 |
| `api/register.rs` | 事务边界 |
| `config.rs` | JWT_SECRET production 强制 |
| `db/commands.rs` | 修复 unwrap_or(0) |
| `stripe/mod.rs` | 消除 unwrap |
| `auth/rate_limit.rs` | 新建 rate limiter |

---

## Part 2: 账户管理 + 邮件通知

### 2.1 密码重置

复用 `email_verifications` 表，新增 `purpose` 列。

| 端点 | 认证 | 流程 |
|------|------|------|
| `POST /api/tenant/forgot-password` | 无 | email → 查 tenant → 生成 6 位码 → 发邮件 |
| `POST /api/tenant/reset-password` | 无 | email + code + new_password → 验证码 → 更新密码 |

### 2.2 邮箱修改

| 端点 | 认证 | 流程 |
|------|------|------|
| `POST /api/tenant/change-email` | JWT | current_password + new_email → 验证密码 → 发码到新邮箱 |
| `POST /api/tenant/confirm-email-change` | JWT | code → 验证 → 更新 email |

### 2.3 租户信息修改

| 端点 | 认证 | 说明 |
|------|------|------|
| `PUT /api/tenant/profile` | JWT | 修改 name |
| `POST /api/tenant/change-password` | JWT | current_password + new_password |

### 2.4 邮件模板

扩展 `email/mod.rs`，新增 6 种邮件：

| 函数 | 触发时机 |
|------|----------|
| `send_password_reset_code` | 密码重置 |
| `send_email_change_code` | 邮箱修改 |
| `send_subscription_activated` | Stripe 订阅激活 |
| `send_subscription_canceled` | Stripe 订阅取消 |
| `send_payment_failed` | 支付失败 |
| `send_refund_processed` | 退款完成 |

所有邮件双语（西班牙语+英语），与现有 `send_verification_code` 风格一致。

### 2.5 数据库变更

Migration：`email_verifications` 表新增 `purpose TEXT NOT NULL DEFAULT 'registration'`。

### 文件清单

| 文件 | 操作 |
|------|------|
| 新 migration | `email_verifications` 加 `purpose` 列 |
| `api/tenant.rs` | 新增 4 个账户管理端点 |
| `api/mod.rs` | 注册新路由 |
| `db/tenants.rs` | 新增 update_password, update_email |
| `db/email_verifications.rs` | 支持 purpose 参数 |
| `email/mod.rs` | 新增 6 种邮件函数 |

---

## Part 3: Stripe 补全

### 3.1 新增 Webhook 事件

| 事件 | 处理逻辑 |
|------|----------|
| `charge.refunded` | 记录退款 + `send_refund_processed` |
| `customer.subscription.trial_will_end` | 试用即将结束邮件提醒 |
| `invoice.paid` | 更新 `current_period_end` + `send_subscription_activated` |
| `invoice.payment_action_required` | 邮件告知需要支付操作 |

### 3.2 Customer Portal

| 端点 | 认证 | 说明 |
|------|------|------|
| `POST /api/tenant/billing-portal` | JWT | 生成 Stripe Billing Portal URL |

### 3.3 stripe/mod.rs 补充

新增 `create_billing_portal_session(secret_key, customer_id, return_url) -> Result<String>`。

退款不主动发起（通过 Stripe Dashboard），只处理 webhook 回调。

### 文件清单

| 文件 | 操作 |
|------|------|
| `api/stripe_webhook.rs` | 新增 4 个事件处理 |
| `api/tenant.rs` | 新增 billing-portal 端点 |
| `stripe/mod.rs` | 新增 billing portal session 函数 |

---

## Part 4: 轻量审计日志

### 4.1 审计表

```sql
CREATE TABLE IF NOT EXISTS cloud_audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    action TEXT NOT NULL,
    detail JSONB,
    ip_address TEXT,
    created_at BIGINT NOT NULL
);
CREATE INDEX idx_cloud_audit_tenant ON cloud_audit_log (tenant_id, created_at);
```

### 4.2 审计操作

| action | 触发时机 |
|--------|----------|
| `login` | 登录成功 |
| `login_failed` | 登录失败 |
| `password_reset` | 密码重置完成 |
| `password_changed` | 密码修改完成 |
| `email_changed` | 邮箱修改完成 |
| `command_created` | 下发远程命令 |
| `subscription_activated` | 订阅激活 |
| `subscription_canceled` | 订阅取消 |

### 4.3 实现

新建 `db/audit.rs`：`pub async fn log(pool, tenant_id, action, detail, ip_address, now)`。在各端点内同步调用。

### 4.4 查询端点

| 端点 | 认证 | 说明 |
|------|------|------|
| `GET /api/tenant/audit-log` | JWT | 查询自己的审计日志（分页）|

### 文件清单

| 文件 | 操作 |
|------|------|
| 新 migration | `cloud_audit_log` 表 |
| `db/audit.rs` | 新建审计写入+查询 |
| `db/mod.rs` | 导出 |
| `api/tenant.rs` | 新增 audit-log 端点 |
| 各端点 | 添加审计调用 |

---

## 实施顺序

1. **Part 1: 生产加固** — 安全基础，最高优先级
2. **Part 4: 审计日志** — 依赖少，为后续操作提供可追踪性
3. **Part 2: 账户管理** — 核心用户功能
4. **Part 3: Stripe 补全** — 最后补全支付场景

## 验证

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace --lib
```
