# crab-cloud 部署指南

## 你需要准备的

### 1. 域名 + DNS

- [ ] 购买域名（如 `crab.es`）
- [ ] 在 AWS Route 53 创建 Hosted Zone
- [ ] 将域名 NS 记录指向 Route 53
- [ ] 规划子域名：
  - `cloud.crab.es` → crab-cloud (ECS/ALB)
  - `auth.crab.es` → crab-auth (Lambda Function URL)

### 2. AWS 账户

- [ ] 创建 AWS 账户（如已有则跳过）
- [ ] 创建 IAM 用户用于 CI/CD 部署，附加最小权限
- [ ] 安装 AWS CLI 并配置 `aws configure`
- [ ] 配置 GitHub OIDC Provider（用于 GitHub Actions 免密部署）

### 3. ACM 证书（HTTPS）

- [ ] 在 ACM 申请证书：`*.crab.es` + `crab.es`
  - 验证方式：DNS（Route 53 可一键添加 CNAME）
  - 等待 Issued 状态
- [ ] 记录 `ACM_CERTIFICATE_ARN`

### 4. SES 邮件服务

- [ ] 在 SES 中验证域名 `crab.es`
  - 添加 DKIM (3 条 CNAME) + SPF (TXT) + DMARC (TXT) DNS 记录
- [ ] **申请脱离 SES 沙箱**（Production Access）
  - AWS Console → SES → Account Dashboard → Request Production Access
  - 审批通常 1-2 天

### 5. Stripe

- [ ] 注册 Stripe 账户（西班牙实体）
- [ ] 创建 Products + Prices（basic €29/月, pro €79/月, enterprise €199/月）
- [ ] 记录 `STRIPE_SECRET_KEY` (sk_live_...)
- [ ] 配置 Webhook:
  - URL: `https://cloud.crab.es/stripe/webhook`
  - 事件: `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`, `invoice.payment_failed`
  - 记录 `STRIPE_WEBHOOK_SECRET` (whsec_...)

### 6. GitHub Repository Secrets

配置以下 GitHub Secrets（Settings → Secrets → Actions）：

| Secret | 说明 |
|--------|------|
| `AWS_DEPLOY_ROLE_ARN` | OIDC IAM Role ARN |
| `LAMBDA_S3_BUCKET` | Lambda 部署包 S3 桶名 |

---

## 首次部署

```bash
# 1. 创建全部基础设施（VPC, RDS, ECS, Lambda, WAF, 告警）
export ACM_CERTIFICATE_ARN="arn:aws:acm:eu-south-2:xxx:certificate/xxx"
export ALERT_EMAIL="your@email.com"
./deploy/deploy.sh setup

# 2. 设置敏感变量（交互式输入 DATABASE_URL, Stripe keys）
./deploy/deploy.sh secrets

# 3. DNS 配置
# 在 Route 53 添加:
#   cloud.crab.es → CNAME → ALB DNS name (从 stack outputs 获取)
#   auth.crab.es  → CNAME → Lambda Function URL (从 stack outputs 获取)

# 4. 验证
curl https://cloud.crab.es/health
./deploy/deploy.sh status
```

## 日常部署

```bash
# 推送到 main 分支后 GitHub Actions 自动部署
git push origin main

# 或手动部署
./deploy/deploy.sh auth     # 更新 crab-auth
./deploy/deploy.sh cloud    # 更新 crab-cloud
./deploy/deploy.sh all      # 更新全部
./deploy/deploy.sh status   # 检查健康状态
```

## 安全架构

```
Internet
    │
    ├─→ WAF (Rate Limit + AWS Managed Rules: SQLi, XSS, Bad Inputs)
    │     │
    │     └─→ ALB (HTTPS, TLS 1.3) ─→ ECS Fargate (crab-cloud)
    │                                      │
    │                                      ├─ Secrets: Secrets Manager
    │                                      ├─ Email: SES
    │                                      └─ DB: RDS (private subnet)
    │
    └─→ Lambda Function URL (crab-auth)
           │
           ├─ PKI: Secrets Manager
           ├─ P12: S3 (KMS encrypted)
           └─ DB: RDS (private subnet, shared)
```

### 关键安全措施

| 措施 | 说明 |
|------|------|
| **Secrets Manager** | 所有敏感配置（DB URL、Stripe keys），不使用明文环境变量 |
| **WAF** | Rate limiting (1000 req/5min) + AWS 托管规则 (SQLi, XSS, Bad Inputs) |
| **VPC Flow Logs** | 记录被拒绝的网络流量，保留 90 天 |
| **RDS 加固** | 加密存储、删除保护、Performance Insights、Enhanced Monitoring |
| **S3 加固** | KMS 加密、版本控制、阻止公共访问 |
| **TLS 1.3** | ALB 强制 TLS 1.3 策略 |
| **HTTP→HTTPS** | 自动 301 重定向 |
| **ECR** | 镜像推送时自动漏洞扫描，不可变标签 |
| **ECS** | 部署断路器 + 自动回滚 |

## 监控告警

告警通过 SNS 邮件通知（配置 `ALERT_EMAIL`）。

| 告警 | 条件 | 严重性 |
|------|------|--------|
| RDS CPU | > 80% 持续 10 分钟 | Warning |
| RDS 存储 | < 2 GB | Critical |
| RDS 连接数 | > 68 (80% max) | Warning |
| ECS 任务 | 运行数 = 0 | Critical |
| Lambda 错误 | > 5 次/5分钟 | Warning |
| Lambda 延迟 | P99 > 10s | Warning |
| ALB 5xx | > 10 次/5分钟 | Warning |
| ALB 延迟 | P95 > 5s | Warning |
| WAF 拦截 | > 100 次/5分钟 | Info (可能攻击) |

## 月费估算（10 家餐厅规模）

| 组件 | 月费 |
|------|------|
| ECS Fargate (0.25 vCPU, 0.5GB) | ~$9 |
| ALB | ~$16 |
| RDS db.t4g.micro | ~$12 |
| NAT Gateway | ~$32 |
| Lambda | ~$0 |
| Secrets Manager (3 secrets) | ~$1.20 |
| WAF | ~$5 |
| Route 53 | ~$0.50 |
| S3, SES, ACM, CloudWatch | ~$0 |
| **总计** | **~$76/月** |

> NAT Gateway 是最大开销。未来可考虑 VPC Endpoints 替代（适用于 Secrets Manager、S3、ECR 等 AWS 服务调用）。

## 推荐区域

`eu-south-2`（西班牙，马德里）— 最低延迟，数据驻留合规。
