# crab-cloud 部署指南

## 你需要准备的

### 1. 域名 + DNS

- [ ] 购买域名（如 `crab.es`）
- [ ] 在 AWS Route 53 创建 Hosted Zone
- [ ] 将域名 NS 记录指向 Route 53
- [ ] 规划子域名：
  - `cloud.crab.es` → crab-cloud (ECS/ALB)
  - `auth.crab.es` → crab-auth (Lambda/API Gateway)

### 2. AWS 账户

- [ ] 创建 AWS 账户（如已有则跳过）
- [ ] 创建 IAM 用户/角色，用于 CI/CD 部署
- [ ] 安装 AWS CLI 并配置 `aws configure`

### 3. RDS PostgreSQL

- [ ] 创建 RDS PostgreSQL 实例（推荐 `db.t4g.micro` 起步）
  - Engine: PostgreSQL 16
  - 存储: 20 GB gp3
  - VPC: 与 ECS 同一 VPC
  - 安全组: 仅允许 ECS 任务访问 5432 端口
- [ ] 记录连接信息 → `DATABASE_URL=postgres://user:pass@host:5432/crab`

### 4. SES 邮件服务（发验证码）

- [ ] 在 SES 中验证域名 `crab.es`
  - 添加 DKIM (3条 CNAME) + SPF (TXT) + DMARC (TXT) DNS 记录
  - 等待验证通过（通常几分钟）
- [ ] **申请脱离 SES 沙箱**（Production Access）
  - AWS Console → SES → Account Dashboard → Request Production Access
  - 说明用途：transactional email for SaaS registration
  - 审批通常 1-2 天
  - 沙箱模式下只能发给已验证邮箱，无法给新用户发验证码
- [ ] 记录：`SES_FROM_EMAIL=noreply@crab.es`

### 5. Stripe

- [ ] 注册 Stripe 账户（[stripe.com](https://stripe.com)，选西班牙实体）
- [ ] 在 Dashboard 创建 Products + Prices：

| Plan | 建议月价 | edge_servers | clients |
|------|---------|-------------|---------|
| Basic | €29/月 | 1 | 5 |
| Pro | €79/月 | 3 | 10 |
| Enterprise | €199/月 | 10 | 50 |

- [ ] 记录 API Keys:
  - `STRIPE_SECRET_KEY=sk_live_...`（或 `sk_test_...` 用于测试）
- [ ] 配置 Webhook:
  - URL: `https://cloud.crab.es/stripe/webhook`
  - 事件: `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`, `invoice.payment_failed`
  - 记录：`STRIPE_WEBHOOK_SECRET=whsec_...`

### 6. ACM 证书（HTTPS）

- [ ] 在 ACM 申请证书：`*.crab.es` + `crab.es`
  - 验证方式：DNS（Route 53 可一键添加）
  - 等待 Issued 状态
- [ ] 此证书用于 ALB HTTPS 终结

### 7. ECR 镜像仓库

- [ ] 创建 ECR 仓库：`crab-cloud`

```bash
aws ecr create-repository --repository-name crab-cloud --region eu-south-2
```

### 8. ECS Fargate 集群

- [ ] 创建 ECS 集群
- [ ] 创建 Task Definition（参考下方环境变量）
- [ ] 创建 Service + ALB Target Group
- [ ] ALB 监听器：443 (HTTPS, ACM 证书) → Target Group (8080)

---

## 环境变量清单

| 变量 | 必需 | 示例 | 说明 |
|------|------|------|------|
| `DATABASE_URL` | Yes | `postgres://crab:xxx@rds-host:5432/crab` | RDS 连接 |
| `HTTP_PORT` | No | `8080` (默认) | HTTP 端口 |
| `MTLS_PORT` | No | `8443` (默认) | mTLS 端口 |
| `ROOT_CA_PATH` | No | `certs/root_ca.pem` | Root CA 文件路径 |
| `SERVER_CERT_PATH` | No | `certs/server.pem` | 服务器 TLS 证书 |
| `SERVER_KEY_PATH` | No | `certs/server.key` | 服务器 TLS 私钥 |
| `ENVIRONMENT` | No | `production` | 环境标识 |
| `SES_FROM_EMAIL` | No | `noreply@crab.es` | 发件人邮箱 |
| `STRIPE_SECRET_KEY` | Yes | `sk_live_...` | Stripe API Key |
| `STRIPE_WEBHOOK_SECRET` | Yes | `whsec_...` | Webhook 签名密钥 |
| `REGISTRATION_SUCCESS_URL` | No | `https://crab.es/registration/success` | 支付成功跳转 |
| `REGISTRATION_CANCEL_URL` | No | `https://crab.es/registration/cancel` | 支付取消跳转 |
| `AWS_REGION` | Yes | `eu-south-2` | AWS 区域（SES + Secrets Manager） |
| `RUST_LOG` | No | `crab_cloud=info` | 日志级别 |

> 敏感变量 (`DATABASE_URL`, `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`) 建议存入 AWS Secrets Manager，通过 ECS Task Definition 的 `secrets` 字段注入。

---

## 构建 & 推送

```bash
# 登录 ECR
aws ecr get-login-password --region eu-south-2 | \
  docker login --username AWS --password-stdin <account-id>.dkr.ecr.eu-south-2.amazonaws.com

# 构建
docker build -t crab-cloud -f crab-cloud/Dockerfile .

# 打标签
docker tag crab-cloud:latest <account-id>.dkr.ecr.eu-south-2.amazonaws.com/crab-cloud:latest

# 推送
docker push <account-id>.dkr.ecr.eu-south-2.amazonaws.com/crab-cloud:latest
```

## 首次部署顺序

1. RDS + 网络（VPC, 安全组）
2. ECR + 推送镜像
3. ECS Task Definition + Service + ALB
4. Route 53 指向 ALB
5. 测试 `https://cloud.crab.es/health`
6. Stripe webhook 配置 + 测试
7. SES 脱离沙箱后测试注册流程

## 推荐 AWS 区域

`eu-south-2`（西班牙，马德里）— 最低延迟，数据驻留合规。
