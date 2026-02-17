# crab-cloud 安全修复 + AWS 架构补全

## 目标

修复代码审查发现的 8 个安全/逻辑问题，同时补全 AWS 基础设施（NLB for mTLS, Secrets Manager 补全），使 crab-cloud 能正确接收 edge-server 的 mTLS 同步连接。

## 架构背景

```
餐厅 LAN                              AWS (eu-south-2)
edge-server → 互联网 → NLB (TCP 8443) → ECS crab-cloud (mTLS 端点)
浏览器/API  → 互联网 → ALB (HTTPS 443) → ECS crab-cloud (HTTP 端点)
```

- edge-server 是局域网边缘服务，**主动**向 crab-cloud 发起 mTLS 连接同步数据
- ALB 是 Layer 7，无法做 TCP 直通 — 需要 NLB 做 TCP pass-through 到 8443
- mTLS 握手在 crab-cloud 进程内完成（rustls），不由 AWS 负载均衡器终止

---

## Part 1: 代码安全修复

### 1.1 提取公共 helpers

`register.rs` 和 `tenant.rs` 中重复的函数提取到 `crab-cloud/src/util.rs`：
- `hash_password(password: &str) -> Result<String>`
- `verify_password(password: &str, hash: &str) -> bool`
- `generate_code() -> String`
- `now_millis() -> i64`
- `error_response(status, msg) -> (StatusCode, Json<Value>)`

### 1.2 X-Forwarded-For 取最后一个 IP

ALB 在 X-Forwarded-For 末尾追加真实 client IP。当前代码取第一个（可伪造）。

修改 `rate_limit.rs::extract_ip()`：取 `rsplit(',').next()`（最后一个）。

### 1.3 生产环境 secrets 强制非空

`config.rs` 中统一处理：`STRIPE_SECRET_KEY`、`STRIPE_WEBHOOK_SECRET`、`JWT_SECRET` 在非 development 环境必须非空，否则 panic。

### 1.4 Webhook 幂等性 INSERT-first

替换 SELECT EXISTS → INSERT 为：
```sql
INSERT INTO processed_webhook_events (event_id, event_type, processed_at)
VALUES ($1, $2, $3) ON CONFLICT DO NOTHING
```
检查 `rows_affected() == 0` 跳过重复事件。移除原有的 SELECT EXISTS 查询。

### 1.5 increment_attempts 错误不可忽略

`register.rs` 和 `tenant.rs` 中 `let _ = increment_attempts(...)` 改为正确的错误处理：失败时返回 500。

### 1.6 confirm_email_change 绑定 tenant_id

在 `change_email` 中将 `tenant_id` 存入 verification record（通过在 email_verifications 表新增 `metadata` TEXT 列，存 JSON `{"tenant_id":"...","old_email":"..."}`）。

`confirm_email_change` 验证时：解析 metadata，确认 `tenant_id` 匹配当前 JWT 的 tenant_id。

### 1.7 认证端点改用 find_by_id

JWT 保护的端点（billing_portal, change_email, change_password, update_profile）中，用 `find_by_id(&state.pool, &identity.tenant_id)` 替代 `find_by_email(&state.pool, &identity.email)`。

### 1.8 Webhook 签名时间戳校验

在 `stripe::verify_webhook_signature` 中：
1. 从 sig_header 解析 `t=` timestamp
2. 检查 `|now - t| <= 300` 秒（Stripe 推荐 5 分钟容忍度）
3. 超时返回错误

---

## Part 2: AWS 架构补全

### 2.1 NLB for mTLS (TCP pass-through)

CloudFormation 新增资源：

**NLB**（Network Load Balancer）：
- Type: `network`
- Scheme: `internet-facing`
- Subnets: PublicSubnet1, PublicSubnet2

**NLBTargetGroup**：
- Protocol: TCP
- Port: 8443
- Target type: ip
- Health check: TCP on port 8443

**NLBListener**：
- Port: 8443
- Protocol: TCP
- Forward to NLBTargetGroup

**ECS 变更**：
- `CrabCloudTaskDef` 新增 PortMapping: 8443/tcp
- `CrabCloudService` 新增 LoadBalancer 条目指向 NLBTargetGroup
- `ECSSecurityGroup` 新增 ingress 规则: 允许 0.0.0.0/0 → 8443（NLB 不用安全组，直接转发）

**Outputs**：
- 新增 `NLBDnsName`，用于 CNAME 配置 `mtls.redcoral.app`

**成本影响**：NLB ~$16/month（与 ALB 类似），总月费约 $92。

### 2.2 Secrets Manager 补全

新增 secret：
- `crab/{Environment}/jwt-secret` — JWT 签名密钥

CloudFormation 变更：
- 新增 `JwtSecret` resource (AWS::SecretsManager::Secret)
- `CrabCloudTaskDef.Secrets` 新增: `JWT_SECRET` → `!Ref JwtSecret`
- `CrabCloudExecutionRole` 的 ReadSecrets policy 新增 `!Ref JwtSecret`

`setup-secrets.sh` 更新：新增 JWT_SECRET 交互式设置。

### 2.3 mTLS 证书挂载

ECS 容器需要访问 server cert + key + root CA。两种方案：

**方案 A（推荐）：Secrets Manager 存证书**
- 将 server.pem、server.key、root_ca.pem 存入 Secrets Manager
- 容器启动时从环境变量读取，写入临时文件
- 需修改 `config.rs` 支持从环境变量直接读 PEM 内容（而非文件路径）

**方案 B：EFS 挂载**
- 创建 EFS 文件系统，存放证书
- ECS task 挂载 EFS volume
- 更复杂，成本更高

选择方案 A：在 `config.rs` 中新增 `ROOT_CA_PEM`、`SERVER_CERT_PEM`、`SERVER_KEY_PEM` 环境变量，直接传 PEM 内容。优先使用这些环境变量，fallback 到文件路径。

### 2.4 Rate Limiter 备注

保持现有 in-memory 实现（ECS DesiredCount=1，暂不需要分布式状态）。在代码中加注释说明扩容时需迁移到 WAF 自定义规则或 ElastiCache。

---

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `crab-cloud/src/util.rs` | **新建** | 公共 helpers |
| `crab-cloud/src/lib.rs` 或 `main.rs` | 修改 | 导出 util |
| `crab-cloud/src/api/register.rs` | 修改 | 删除重复 helpers，引用 util，修复 increment_attempts |
| `crab-cloud/src/api/tenant.rs` | 修改 | 删除重复 helpers，引用 util，find_by_id，bind tenant_id |
| `crab-cloud/src/api/stripe_webhook.rs` | 修改 | INSERT-first 幂等性 |
| `crab-cloud/src/auth/rate_limit.rs` | 修改 | X-Forwarded-For 取最后 IP |
| `crab-cloud/src/config.rs` | 修改 | secrets 强制非空 + PEM 环境变量支持 |
| `crab-cloud/src/main.rs` | 修改 | 导出 util 模块 + PEM 从 env 读取 |
| `crab-cloud/src/stripe/mod.rs` | 修改 | 签名时间戳校验 |
| `crab-cloud/src/db/email_verifications.rs` | 修改 | 新增 metadata 列支持 |
| `crab-cloud/migrations/0006_email_verification_metadata.up.sql` | **新建** | metadata 列 |
| `deploy/cloudformation.yml` | 修改 | NLB + JwtSecret + 8443 端口 + PEM secrets |
| `deploy/setup-secrets.sh` | 修改 | JWT_SECRET + 证书 PEM 设置 |
