# Crab 全栈生产环境审计报告

**日期**: 2026-02-24
**评估范围**: 7 个 workspace crate + 前端 + CI/CD + 部署
**评估方法**: 4 个并行 agent 深度审计（crab-cloud, edge-server, shared, red_coral, CI/CD）

---

## 综合评分

| 维度 | 评分 | 说明 |
|------|------|------|
| 核心业务逻辑 | 9/10 | 订单事件溯源测试 9000+ 行，设计出色 |
| 错误处理体系 | 8/10 | 统一 ErrorCode 117 变体，执行一致 |
| 安全 | 5/10 | mTLS/PKI 强，私钥管理/CORS/DevTools 有漏洞 |
| 类型安全 | 5/10 | 大量 String-where-enum-should-be |
| 金额处理 | 7/10 | 存储用 f64 合规，前端有少量浮点计算违规 |
| CI/CD | 4/10 | crab-cloud 排除 CI，无安全扫描 |
| 可观测性 | 3/10 | 日志规范好，无 metrics/告警/监控 |
| 部署安全 | 3/10 | Root CA 私钥明文存储 |

---

## P0 — 必须立即修复

### 1. Root CA 私钥明文存储 (deploy/ec2/certs/root_ca_secret.json)
- 权限 0644，拿到即可伪造所有 tenant mTLS 证书
- **修复**: 删除本地文件，仅保留 AWS Secrets Manager

### 2. state_checksum 使用 DefaultHasher (shared/src/order/snapshot.rs:305-328)
- 跨 Rust 版本不稳定，滚动更新时误报
- **修复**: 改用确定性哈希（CRC32 或 SHA256）

### 4. crab-cloud 无优雅关闭 (crab-cloud/src/main.rs)
- SIGTERM 杀进程，WS 同步/Stripe webhook/DB 事务中断
- **修复**: `axum::serve(...).with_graceful_shutdown()`

### 5. Stripe 订阅状态默认 Active (stripe_webhook.rs:236-244)
- `"incomplete"`/`"trialing"` 等未付款状态默认 Active
- **修复**: 默认改为 Suspended，显式枚举所有状态

---

## P1 — 高风险 (28 项)

### 安全 (5)
| # | 问题 | 位置 |
|---|------|------|
| 6 | JWT secret 写入权限 0644 | edge-server/src/auth/jwt.rs:198 |
| 7 | devtools + withGlobalTauri 暴露 Tauri 命令 | tauri.conf.json:13,21 |
| 8 | signable_data() 管道分隔符可注入 | shared/src/activation.rs:146-157 |
| 9 | SSH StrictHostKeyChecking=no | deploy/sync-*.sh:38 |
| 10 | Stripe Price ID 硬编码生产默认值 | crab-cloud/src/config.rs:111-117 |

### 可靠性 (7)
| # | 问题 | 位置 |
|---|------|------|
| 11 | Edge WS 无服务端 Ping | crab-cloud/src/api/ws.rs:111-161 |
| 12 | SyncBatch 无批次大小上限 | crab-cloud/src/api/ws.rs:219-265 |
| 13 | PgPool 默认配置无超时 | crab-cloud/src/state.rs:118 |
| 14 | TCP handshake 无超时可 DoS | edge-server/src/message/tcp_server.rs:252 |
| 15 | DB 错误被 unwrap_or_default 吞掉 | edge-server/src/api/orders/handler.rs:321,343,402 |
| 16 | initializeOrders 失败 POS 冻结无恢复 | red_coral useOrderEventListener.ts:51-72 |
| 17 | 后台任务 panic 不重启 | edge-server/src/core/tasks.rs:111-138 |

### 类型安全 (10)
| # | 问题 | 位置 |
|---|------|------|
| 18 | PaymentMethod 是自由 String | shared/src/order/types.rs:255 |
| 19 | SyncPayload.action 是 String | shared/src/message/payload.rs:175 |
| 20 | OrderDetailSync.status 是 String | shared/src/cloud/sync.rs:214 |
| 21 | CloudSyncItem.resource_id 是 String | shared/src/cloud/sync.rs:172 |
| 22 | Event schema 无版本号 | shared/src/order/event.rs |
| 23 | ResponsePayload.error_code 是 Option\<String\> | shared/src/message/payload.rs:197 |
| 24 | delete_resource 绑定类型错误 | crab-cloud/src/db/sync_store.rs:479-495 |
| 25 | void_type/loss_reason 降级为 String | shared/src/cloud/sync.rs:269-270 |
| 26 | OrderEventSync.event_type 是 String | shared/src/cloud/sync.rs:242-248 |
| 27 | HandshakePayload 无版本协商响应 | shared/src/message/payload.rs:106-116 |

### 前端 (4)
| # | 问题 | 位置 |
|---|------|------|
| 28 | calculateOptionsModifier 用原生 float | red_coral/src/utils/pricing.ts:17-23 |
| 29 | Price rule 输入用 Math.round 陷阱 | Step2Adjustment.tsx:43-44 |
| 30 | handleAuthError 不清后端 session | tauri-client.ts:84-97 |
| 31 | 离线登录 7 天窗口（离职员工风险） | session_cache.rs:200-208 |

### CI/CD (2)
| # | 问题 | 位置 |
|---|------|------|
| 32 | crab-cloud 排除 CI | .github/workflows/rust.yml:47,50 |
| 33 | 无安全扫描 | .github/workflows/ |

---

## P2 — 工程质量改进 (46 项)

### 安全 (7)
- CORS permissive 应收紧 | edge-server/src/services/https.rs:98
- CSP 允许任意 WebSocket | tauri.conf.json:25
- assetProtocol.scope 允许任意文件 | tauri.conf.json:27-30
- ActivationData.entity_key 泄露到 Debug | shared/src/activation.rs:80
- JWT 30 天无撤销 | edge-server/src/auth/jwt.rs:54
- verify_email TOCTOU 竞态 | crab-cloud/register.rs:221-234
- QuotaCache 无上限无清理 | crab-cloud/quota.rs:30-38

### 可靠性 (10)
- Session cache save 非原子写入 | session_cache.rs:130-133
- payment_id Date.now() 可碰撞 | usePaymentActions.ts:85
- 重复票号风险 | orders/manager/mod.rs:152
- Stamp tracking race condition | orders/manager/mod.rs:953-957
- 非 Welcome WS 消息终止 session | cloud/worker.rs:426-430
- 端口配置无效静默回退 | crab-cloud/config.rs:76-83
- HTTPS graceful shutdown 仅 2s | edge-server/services/https.rs:150
- rate_limit 全局 Mutex | crab-cloud/rate_limit.rs:22
- Argon2 阻塞 tokio | crab-cloud/register.rs:89-95
- escalate 字符串匹配错误 | red_coral/commands/auth.rs:90-99

### 类型安全 (11)
- zone_scope 混合类型 String | applied_rule.rs:23
- 时间范围未验证 HH:MM String | price_rule.rs:70-71
- CartItemInput 接受负数 | types.rs:137-167
- SplitItem 全部 serde(default) | types.rs:238-250
- billing_interval Option\<String\> | activation.rs:310
- features Vec\<String\> 无类型 | activation.rs:298
- OrderEventSync.data JSON-in-string | cloud/sync.rs:247
- ApiResponse.code Option\<u16\> | error/types.rs:237
- remaining_amount 方法/字段分歧 | snapshot.rs:284
- ErrorCode http_status catch-all | error/http.rs:123
- PKI 路由自造 JSON | crab-cloud/pki/*.rs

### 前端 (8)
- useOrderByTable 无 useShallow | useActiveOrdersStore.ts:378
- useActiveOrderCount O(n) 扫描 | useActiveOrdersStore.ts:397
- 硬编码 EUR/euro 符号 | 全局 30+ 处
- || 而非 ?? 导致 0 穿透 | pricing.ts:42
- hasPermission 硬编码 'admin' | useAuthStore.ts:124
- PaymentMethod 类型失效 | orderEvent.ts:1016
- persist 空 partialize 死代码 | useBridgeStore.ts:476
- bulkDeleteProducts 串行删除 | tauri-client.ts:240-244

### CI/CD (10)
- Actions 用浮动 tag | workflows/*.yml
- 缺 permissions 和 concurrency | rust.yml
- Dockerfile 浮动 tag | crab-cloud/Dockerfile:2
- console/portal 无 CI | workflows/
- Caddyfile 缺 HSTS/CSP | deploy/ec2/Caddyfile
- 日志不分域名 | Caddyfile
- 无监控告警 | deploy/ 全局
- docker-compose v1 EOL | deploy/ 全局
- mTLS cert 2028 到期无轮换 | deploy/ec2/certs/
- .env.example 缺 5 个变量 | deploy/ec2/.env.example

---

## 推荐修复顺序

### Phase 1 — 安全紧急 (本周)
- 删除本地 Root CA 私钥文件
- crab-cloud 添加 graceful shutdown
- 修复 Stripe 订阅状态默认值
- 关闭生产 devtools + withGlobalTauri
- JWT secret 改用 write_secret_file (0600)

### Phase 2 — 数据正确性 (1-2 周)
- state_checksum 确定性哈希
- String → Enum 类型强化
- delete_resource 绑定类型修复
- 前端浮点计算改用 Currency 类（calculateOptionsModifier, Math.round 等）

### Phase 3 — 可靠性加固 (2-3 周)
- PgPool 配置
- Edge WS Ping + SyncBatch 上限
- TCP handshake 超时
- 前端 initializeOrders 重试
- 后台任务自动重启
- DB 错误不再被吞掉

### Phase 4 — CI/CD 补齐 (2-3 周)
- crab-cloud 加入 CI
- cargo audit + npm audit
- 部署回滚脚本
- 备份验证 + 告警
- Actions 固定 SHA

### Phase 5 — 可观测性 (持续)
- 外部 uptime 监控
- Prometheus metrics / CloudWatch
- 错误告警 (Sentry / Slack)
- 日志按域名分离
