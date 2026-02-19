# Crab SaaS 上线设计文档

**日期**: 2026-02-18
**目标**: 内测上线（云端服务 + CI/CD）
**范围**: crab-auth (Lambda) + crab-cloud (ECS Fargate)，客户端分发暂不涉及

## 现有基础设施

| 组件 | 状态 | 文件 |
|------|------|------|
| CloudFormation 模板 | ✅ 完整 | `deploy/cloudformation.yml` |
| CI/CD (GitHub Actions) | ✅ 完整 | `.github/workflows/ci.yml` |
| Release 流水线 | ✅ 完整 | `.github/workflows/release.yml` |
| 部署脚本 | ✅ 完整 | `deploy/deploy.sh` + `build-*.sh` + `setup-secrets.sh` |
| 安全加固 | ✅ 已完成 | 9 项 P0 修复（IP 提取、幂等性、时间戳验证、租户隔离等） |

## 部署架构

```
Cloudflare DNS
  ├── cloud.域名 → ALB (HTTPS 443) → ECS Fargate (crab-cloud :8080)
  ├── sync.域名  → NLB (TCP 8443)  → ECS Fargate (crab-cloud :8443 mTLS)
  └── auth.域名  → Lambda Function URL (crab-auth)
                         │
                     ┌───┴───┐
                     │  RDS  │ PostgreSQL 16 (private subnet)
                     └───────┘
```

## 环境信息

- **AWS 区域**: eu-south-2 (Spain)
- **DNS**: Cloudflare 管理
- **Stripe**: 已有账号
- **SES**: 待配置
- **上线模式**: 内测（自己 + 几家熟人餐厅）

## 执行步骤

### 第 1 步：AWS OIDC + Deploy Role

GitHub Actions 通过 OIDC 认证 AWS（无需长期 Access Key）。

**操作**:
1. 在 AWS IAM 创建 OIDC Identity Provider (`token.actions.githubusercontent.com`)
2. 创建 IAM Role `crab-github-deploy`，信任策略限定到你的 repo
3. 附加权限：ECR push、ECS update、Lambda update、S3 upload

**可选**: 写一个独立的 CloudFormation 模板自动创建这些资源。

### 第 2 步：ACM 证书

**操作**:
1. AWS ACM (eu-south-2) → Request certificate
2. 域名：`*.你的域名` + `你的域名`
3. 验证方式：DNS 验证
4. ACM 给出 CNAME 记录 → 添加到 Cloudflare DNS
5. 等待状态变为 `Issued`（通常 5-10 分钟）

### 第 3 步：创建 Lambda S3 Bucket

```bash
aws s3 mb s3://crab-deploy-artifacts --region eu-south-2
```

### 第 4 步：部署 CloudFormation 栈

```bash
ACM_CERTIFICATE_ARN=arn:aws:acm:eu-south-2:xxx:certificate/xxx \
ALERT_EMAIL=你的邮箱 \
./deploy/deploy.sh setup
```

**创建的资源**:
- VPC (10.0.0.0/16) + 公有/私有子网 + NAT Gateway
- RDS PostgreSQL 16 (db.t4g.micro, 20GB, 加密, 14 天备份)
- ECS Fargate Cluster + crab-cloud Service
- Lambda crab-auth (arm64, 256MB)
- ALB (HTTPS) + NLB (TCP 8443 mTLS 透传)
- WAF v2 (限速 + AWS 托管规则)
- ECR Repository
- S3 P12 证书桶 (KMS 加密)
- Secrets Manager (4 个密钥)
- CloudWatch Alarms (9 个)
- SNS 告警通知

**预估时间**: 15-25 分钟

### 第 5 步：配置 Secrets

```bash
./deploy/deploy.sh secrets
```

交互式输入 4 个值:
- **DATABASE_URL**: `postgres://crab:密码@RDS端点:5432/crab` (RDS 密码从 AWS Secrets Manager 的 RDS 管理密钥获取)
- **STRIPE_SECRET_KEY**: `sk_live_...` 或内测用 `sk_test_...`
- **STRIPE_WEBHOOK_SECRET**: `whsec_...` (Stripe Dashboard → Webhooks 获取)
- **JWT_SECRET**: `openssl rand -hex 32` 生成

### 第 6 步：数据库迁移

RDS 在私有子网，无法直接连接。选项:
- **方案 A (推荐)**: 通过 ECS exec 进入容器执行迁移
- **方案 B**: 创建临时 EC2 跳板机
- **方案 C**: 修改 crab-cloud 启动时自动运行迁移

crab-cloud 的 PostgreSQL migrations 在 `crab-cloud/migrations/` 目录。

### 第 7 步：SES 邮件配置

**操作**:
1. AWS SES (eu-south-2) → Verified identities → 添加域名
2. SES 提供 3 条 DKIM CNAME 记录 → 添加到 Cloudflare
3. 可选：添加 SPF TXT 记录 (`v=spf1 include:amazonses.com ~all`)
4. 内测阶段可以不移出沙箱（只需在 SES 中验证收件人邮箱即可）

### 第 8 步：Cloudflare DNS 配置

CloudFormation Outputs 会输出 ALB/NLB/Lambda 的 DNS 名称。

| 类型 | 名称 | 值 | Proxy 状态 |
|------|------|-----|-----------|
| CNAME | `cloud` | ALB DNS name | ❌ 关闭 (DNS only) |
| CNAME | `sync` | NLB DNS name | ❌ 关闭 (TCP 透传) |
| CNAME | `auth` | Lambda Function URL hostname | ❌ 关闭 |

**重要**: 所有子域名的 Cloudflare Proxy 都建议关闭：
- `sync`: **必须**关闭，NLB TCP 透传 + mTLS，Cloudflare 代理会中断 TLS
- `cloud`: 建议关闭，ALB 已有 WAF + HTTPS，双重代理增加延迟且干扰 X-Forwarded-For
- `auth`: Lambda Function URL 自带 HTTPS

### 第 9 步：GitHub Secrets 配置

在 GitHub repo Settings → Secrets and variables → Actions：

| Secret 名称 | 值 | 内测必需 |
|-------------|-----|---------|
| `AWS_DEPLOY_ROLE_ARN` | 第 1 步创建的 Role ARN | ✅ |
| `LAMBDA_S3_BUCKET` | `crab-deploy-artifacts` | ✅ |
| `TAURI_SIGNING_PRIVATE_KEY` | Tauri 签名密钥 | ❌ 后续 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 签名密码 | ❌ 后续 |
| `UPDATE_S3_BUCKET` | 更新文件桶 | ❌ 后续 |
| `UPDATE_DOWNLOAD_BASE_URL` | 下载 base URL | ❌ 后续 |

### 第 10 步：Stripe Webhook 配置

1. Stripe Dashboard → Developers → Webhooks
2. 添加 endpoint: `https://cloud.你的域名/stripe-webhook`
3. 订阅事件: `checkout.session.completed`, `customer.subscription.*`
4. 获取 Webhook Secret → 已在第 5 步配入 Secrets Manager

### 第 11 步：冒烟测试

```bash
# Health check
curl https://cloud.你的域名/health

# Lambda
curl https://auth.你的域名/

# 注册租户
curl -X POST https://cloud.你的域名/register \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"...","restaurant_name":"测试餐厅"}'

# Stripe webhook 本地测试
stripe listen --forward-to https://cloud.你的域名/stripe-webhook

# 部署状态检查
./deploy/deploy.sh status
```

## 代码改动

大部分是 AWS 控制台 + CLI 操作，代码层面可能需要：

1. **CloudFormation 域名参数**: 当前默认 `redcoral.app`，如果域名不同需要部署时覆盖 `DomainName` 参数
2. **OIDC CloudFormation 模板** (可选): 自动化创建 GitHub OIDC Provider + Deploy Role
3. **启动时自动迁移** (可选): crab-cloud 启动时检查并运行 pending migrations

## 预估成本 (内测阶段)

| 服务 | 月费估算 |
|------|---------|
| RDS db.t4g.micro | ~$15 |
| ECS Fargate (256 CPU / 512 MB × 1 task) | ~$10 |
| NAT Gateway | ~$35 |
| ALB | ~$18 |
| NLB | ~$18 |
| Lambda (低频) | ~$1 |
| S3 + Secrets Manager | ~$2 |
| **合计** | **~$99/月** |

> NAT Gateway 是最大开销。如果想省钱，可以考虑用 VPC Endpoints 替代 NAT（但需要改 CloudFormation）。

## 风险与注意

1. **RDS 密码**: CloudFormation 用 `ManageMasterUserPassword: true`，密码自动管理在 Secrets Manager，需要从 RDS 管理密钥里取出来拼 DATABASE_URL
2. **ECR IMMUTABLE tags**: 当前配置 `ImageTagMutability: IMMUTABLE`，ci.yml 用 commit SHA 作 tag（正确），但也 push `latest` tag — 第二次部署 `latest` 会失败。需要改为 MUTABLE 或只用 SHA tag
3. **SES 沙箱限制**: 沙箱模式下只能向已验证邮箱发信，内测够用但正式上线前需要申请移出
4. **Cloudflare 与 AWS 证书**: Cloudflare Proxy 开启时会用自己的证书，关闭时用 ACM 证书。保持一致最简单的方式是全部关闭 Proxy
