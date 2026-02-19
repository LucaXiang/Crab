# Crab SaaS 上线实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 crab-auth (Lambda) + crab-cloud (ECS Fargate) 部署到 AWS eu-south-2，完成内测上线。

**Architecture:** CloudFormation 一键部署 VPC/RDS/ECS/Lambda/ALB/NLB/WAF，GitHub Actions CI/CD 自动构建部署，Cloudflare 管理 DNS。crab-cloud 启动时自动运行 PostgreSQL 迁移 (`state.rs:127`)。

**Tech Stack:** AWS CloudFormation, ECS Fargate, Lambda (arm64), RDS PostgreSQL 16, ALB/NLB, WAF v2, Secrets Manager, SES, ECR, S3, GitHub Actions OIDC, Cloudflare DNS

---

## 前置条件

- AWS 账号已有，区域 eu-south-2
- 域名已购，Cloudflare 管理 DNS
- Stripe 账号已有
- GitHub repo 已有 CI/CD workflows

## 已知 Bug：必须先修

ECR 配置了 `ImageTagMutability: IMMUTABLE`，但 `ci.yml` 里 push `latest` tag，第二次部署会失败。

---

### Task 1: 修复 ECR Immutable Tag 冲突

**Files:**
- Modify: `deploy/cloudformation.yml:1115`
- Modify: `.github/workflows/ci.yml:138-143`

**Step 1: 修改 CloudFormation — ECR 改为 MUTABLE**

在 `deploy/cloudformation.yml` 第 1115 行，将：
```yaml
      ImageTagMutability: IMMUTABLE
```
改为：
```yaml
      ImageTagMutability: MUTABLE
```

**Step 2: 修改 ci.yml — 移除 latest tag push**

ci.yml 已经用 commit SHA 作为唯一 tag 并在 ECS 部署时动态更新 task definition，`latest` tag 是多余的。

在 `.github/workflows/ci.yml`，将 `Build and push Docker image` step 的 run 部分：
```yaml
        run: |
          docker build -f crab-cloud/Dockerfile -t $ECR_REGISTRY/crab-cloud:$IMAGE_TAG .
          docker push $ECR_REGISTRY/crab-cloud:$IMAGE_TAG
          # Also tag as latest
          docker tag $ECR_REGISTRY/crab-cloud:$IMAGE_TAG $ECR_REGISTRY/crab-cloud:latest
          docker push $ECR_REGISTRY/crab-cloud:latest
```
改为：
```yaml
        run: |
          docker build -f crab-cloud/Dockerfile -t $ECR_REGISTRY/crab-cloud:$IMAGE_TAG .
          docker push $ECR_REGISTRY/crab-cloud:$IMAGE_TAG
```

**Step 3: 同步修改 build-cloud.sh — 支持自定义 IMAGE_TAG**

`deploy/build-cloud.sh` 已经支持 `IMAGE_TAG` 环境变量，默认 `latest`。不需要改动，手动部署时可以用：
```bash
IMAGE_TAG=$(git rev-parse --short HEAD) ./deploy/build-cloud.sh push
```

**Step 4: 验证**

```bash
cargo clippy --workspace -- -D warnings
```
Expected: 0 warnings, 0 errors (这些改动不影响 Rust 代码)

**Step 5: Commit**

```bash
git add deploy/cloudformation.yml .github/workflows/ci.yml
git commit -m "fix(deploy): use mutable ECR tags, remove redundant latest push"
```

---

### Task 2: 创建 GitHub OIDC Deploy Role 的 CloudFormation 模板

CI/CD 需要 AWS OIDC 认证。创建一个独立模板，运行一次即可。

**Files:**
- Create: `deploy/github-oidc.yml`

**Step 1: 创建 OIDC CloudFormation 模板**

创建 `deploy/github-oidc.yml`：

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: >
  GitHub Actions OIDC Provider + Deploy Role for Crab SaaS.
  Run ONCE per AWS account: aws cloudformation deploy --template-file deploy/github-oidc.yml --stack-name crab-github-oidc --capabilities CAPABILITY_IAM --parameter-overrides GitHubOrg=YOUR_ORG GitHubRepo=YOUR_REPO

Parameters:
  GitHubOrg:
    Type: String
    Description: GitHub organization or username

  GitHubRepo:
    Type: String
    Description: GitHub repository name

Resources:
  GitHubOIDCProvider:
    Type: AWS::IAM::OIDCProvider
    Properties:
      Url: https://token.actions.githubusercontent.com
      ClientIdList:
        - sts.amazonaws.com
      ThumbprintList:
        - 6938fd4d98bab03faadb97b34396831e3780aea1
        - 1c58a3a8518e8759bf075b76b750d4f2df264fcd
      Tags:
        - Key: Project
          Value: crab

  GitHubDeployRole:
    Type: AWS::IAM::Role
    Properties:
      RoleName: crab-github-deploy
      AssumeRolePolicyDocument:
        Version: '2012-10-17'
        Statement:
          - Effect: Allow
            Principal:
              Federated: !Ref GitHubOIDCProvider
            Action: sts:AssumeRoleWithWebIdentity
            Condition:
              StringEquals:
                token.actions.githubusercontent.com:aud: sts.amazonaws.com
              StringLike:
                token.actions.githubusercontent.com:sub: !Sub repo:${GitHubOrg}/${GitHubRepo}:ref:refs/heads/main
      Policies:
        - PolicyName: CrabDeploy
          PolicyDocument:
            Version: '2012-10-17'
            Statement:
              # ECR: push images
              - Effect: Allow
                Action:
                  - ecr:GetAuthorizationToken
                Resource: '*'
              - Effect: Allow
                Action:
                  - ecr:BatchCheckLayerAvailability
                  - ecr:GetDownloadUrlForLayer
                  - ecr:BatchGetImage
                  - ecr:PutImage
                  - ecr:InitiateLayerUpload
                  - ecr:UploadLayerPart
                  - ecr:CompleteLayerUpload
                Resource: !Sub arn:aws:ecr:${AWS::Region}:${AWS::AccountId}:repository/crab-cloud
              # ECS: deploy
              - Effect: Allow
                Action:
                  - ecs:DescribeTaskDefinition
                  - ecs:RegisterTaskDefinition
                  - ecs:UpdateService
                  - ecs:DescribeServices
                  - ecs:ListTasks
                  - ecs:DescribeTasks
                Resource: '*'
              # ECS: pass roles to task
              - Effect: Allow
                Action:
                  - iam:PassRole
                Resource:
                  - !Sub arn:aws:iam::${AWS::AccountId}:role/*crab*
              # Lambda: update function code
              - Effect: Allow
                Action:
                  - lambda:UpdateFunctionCode
                  - lambda:GetFunction
                  - lambda:GetFunctionConfiguration
                Resource: !Sub arn:aws:lambda:${AWS::Region}:${AWS::AccountId}:function:crab-auth
              # S3: upload Lambda zip
              - Effect: Allow
                Action:
                  - s3:PutObject
                  - s3:GetObject
                Resource: arn:aws:s3:::crab-deploy-artifacts/*
      Tags:
        - Key: Project
          Value: crab

Outputs:
  DeployRoleArn:
    Description: Set this as GitHub Secret AWS_DEPLOY_ROLE_ARN
    Value: !GetAtt GitHubDeployRole.Arn
```

**Step 2: 验证模板语法**

```bash
aws cloudformation validate-template --template-body file://deploy/github-oidc.yml --region eu-south-2
```
Expected: 输出 Parameters 列表，无错误

**Step 3: Commit**

```bash
git add deploy/github-oidc.yml
git commit -m "infra(deploy): add GitHub OIDC provider CloudFormation template"
```

---

### Task 3: 添加 ECS Exec 支持（用于调试和手动操作）

内测阶段需要能 exec 进 ECS 容器查看日志、调试问题。

**Files:**
- Modify: `deploy/cloudformation.yml` (ECS Service + Task Role)

**Step 1: 在 CloudFormation 中启用 ECS Exec**

在 `deploy/cloudformation.yml` 的 `CrabCloudService` 资源 (约第 1068 行) 添加 `EnableExecuteCommand`:

在 `CrabCloudService` Properties 中，`DesiredCount: 1` 下方添加：
```yaml
      EnableExecuteCommand: true
```

**Step 2: 给 Task Role 添加 SSM 权限**

在 `CrabCloudTaskRole` 的 Policies Statement 中添加（在 SES 权限后面）：
```yaml
              # ECS Exec (SSM)
              - Effect: Allow
                Action:
                  - ssmmessages:CreateControlChannel
                  - ssmmessages:CreateDataChannel
                  - ssmmessages:OpenControlChannel
                  - ssmmessages:OpenDataChannel
                Resource: '*'
```

**Step 3: 验证**

```bash
aws cloudformation validate-template --template-body file://deploy/cloudformation.yml --region eu-south-2
```
Expected: 无错误

**Step 4: Commit**

```bash
git add deploy/cloudformation.yml
git commit -m "infra(deploy): enable ECS Exec for debugging"
```

---

### Task 4: 部署 GitHub OIDC 栈（AWS 操作）

**这是手动 AWS 操作，不是代码改动。**

**Step 1: 创建 Lambda S3 Bucket**

```bash
aws s3 mb s3://crab-deploy-artifacts --region eu-south-2
```
Expected: `make_bucket: crab-deploy-artifacts`

**Step 2: 部署 OIDC 栈**

```bash
aws cloudformation deploy \
  --template-file deploy/github-oidc.yml \
  --stack-name crab-github-oidc \
  --capabilities CAPABILITY_IAM \
  --region eu-south-2 \
  --parameter-overrides \
    GitHubOrg=你的GitHub用户名 \
    GitHubRepo=你的repo名
```
Expected: Stack 创建成功

**Step 3: 获取 Role ARN**

```bash
aws cloudformation describe-stacks \
  --stack-name crab-github-oidc \
  --query 'Stacks[0].Outputs[?OutputKey==`DeployRoleArn`].OutputValue' \
  --output text \
  --region eu-south-2
```
Expected: `arn:aws:iam::XXXX:role/crab-github-deploy`

**Step 4: 设置 GitHub Secret**

在 GitHub repo → Settings → Secrets and variables → Actions 中添加：
- `AWS_DEPLOY_ROLE_ARN` = 上一步输出的 ARN
- `LAMBDA_S3_BUCKET` = `crab-deploy-artifacts`

---

### Task 5: 申请 ACM 证书（AWS + Cloudflare 操作）

**Step 1: 申请证书**

```bash
aws acm request-certificate \
  --domain-name "你的域名" \
  --subject-alternative-names "*.你的域名" \
  --validation-method DNS \
  --region eu-south-2
```
Expected: 输出 CertificateArn

**Step 2: 获取 DNS 验证记录**

```bash
aws acm describe-certificate \
  --certificate-arn 上一步的ARN \
  --query 'Certificate.DomainValidationOptions[0].ResourceRecord' \
  --output table \
  --region eu-south-2
```
Expected: 输出一条 CNAME 记录 (Name + Value)

**Step 3: 在 Cloudflare 添加验证 CNAME**

Cloudflare Dashboard → DNS → Add record:
- Type: CNAME
- Name: ACM 给的 Name（去掉域名后缀部分）
- Target: ACM 给的 Value
- Proxy: ❌ DNS only

**Step 4: 等待验证通过**

```bash
aws acm wait certificate-validated \
  --certificate-arn 上一步的ARN \
  --region eu-south-2
```
Expected: 命令成功退出（可能需要 5-10 分钟）

记录下 `CertificateArn`，后面 Task 7 要用。

---

### Task 6: 配置 SES 邮件服务（AWS + Cloudflare 操作）

**Step 1: 验证域名**

```bash
aws sesv2 create-email-identity \
  --email-identity 你的域名 \
  --region eu-south-2
```
Expected: 输出 DKIM tokens

**Step 2: 获取 DKIM 记录**

```bash
aws sesv2 get-email-identity \
  --email-identity 你的域名 \
  --query 'DkimAttributes.Tokens' \
  --output text \
  --region eu-south-2
```
Expected: 3 个 token 字符串

**Step 3: 在 Cloudflare 添加 3 条 DKIM CNAME**

对每个 token，添加 CNAME：
- Name: `{token}._domainkey`
- Target: `{token}.dkim.amazonses.com`
- Proxy: ❌ DNS only

**Step 4: (可选) 添加 SPF TXT 记录**

- Type: TXT
- Name: `@`
- Value: `v=spf1 include:amazonses.com ~all`

**Step 5: 内测阶段 — 验证收件人邮箱**

沙箱模式只能给已验证邮箱发信。验证你自己的邮箱：
```bash
aws sesv2 create-email-identity \
  --email-identity your-email@example.com \
  --region eu-south-2
```
然后去邮箱点击验证链接。

---

### Task 7: 部署主 CloudFormation 栈（AWS 操作）

**这是最关键的一步，创建所有 AWS 基础设施。**

**Step 1: 部署栈**

```bash
ACM_CERTIFICATE_ARN=arn:aws:acm:eu-south-2:xxx:certificate/xxx \
ALERT_EMAIL=你的告警邮箱 \
./deploy/deploy.sh setup
```

这会：
1. 构建 crab-auth Lambda zip（Docker 交叉编译）
2. 上传到 S3
3. 构建 crab-cloud Docker 镜像
4. Push 到 ECR
5. 部署 CloudFormation 栈（15-25 分钟）

Expected: Stack outputs 表格显示 ALB DNS、NLB DNS、Lambda URL、RDS 端点等

**Step 2: 记录输出值**

记录下：
- `ALBDnsName` — 用于 Cloudflare DNS
- `NLBDnsName` — 用于 Cloudflare DNS
- `CrabAuthFunctionUrl` — 用于 Cloudflare DNS
- `RDSEndpoint` — 用于拼 DATABASE_URL

---

### Task 8: 配置 Secrets（AWS 操作）

**Step 1: 获取 RDS 管理密码**

CloudFormation 用 `ManageMasterUserPassword: true`，密码在 Secrets Manager 自动管理。找到它：

```bash
aws secretsmanager list-secrets \
  --filter Key=name,Values=rds \
  --query 'SecretList[].Name' \
  --region eu-south-2
```
然后获取密码：
```bash
aws secretsmanager get-secret-value \
  --secret-id 上面找到的secret名 \
  --query 'SecretString' \
  --output text \
  --region eu-south-2
```
从 JSON 中提取 `password` 字段。

**Step 2: 生成 JWT Secret**

```bash
openssl rand -hex 32
```
记录输出。

**Step 3: 运行 secrets 脚本**

```bash
./deploy/deploy.sh secrets
```

交互式输入：
1. **DATABASE_URL**: `postgres://crab:RDS密码@RDS端点:5432/crab`
2. **STRIPE_SECRET_KEY**: `sk_test_...`（内测用测试密钥）
3. **STRIPE_WEBHOOK_SECRET**: 暂时输入占位值，Task 10 获取真实值后再更新
4. **JWT_SECRET**: Step 2 生成的值

**Step 4: 重启 ECS 服务使 Secrets 生效**

```bash
aws ecs update-service \
  --cluster crab-production \
  --service crab-cloud \
  --force-new-deployment \
  --region eu-south-2 > /dev/null

aws ecs wait services-stable \
  --cluster crab-production \
  --services crab-cloud \
  --region eu-south-2
```
Expected: Service 稳定运行

---

### Task 9: 配置 Cloudflare DNS

**Step 1: 添加 DNS 记录**

在 Cloudflare Dashboard → 你的域名 → DNS：

| Type | Name | Target | Proxy |
|------|------|--------|-------|
| CNAME | `cloud` | Task 7 输出的 ALBDnsName | ❌ DNS only |
| CNAME | `sync` | Task 7 输出的 NLBDnsName | ❌ DNS only |
| CNAME | `auth` | Task 7 输出的 CrabAuthFunctionUrl (去掉 `https://` 和尾部 `/`) | ❌ DNS only |

**关键：** 所有记录的 Proxy 状态必须关闭（灰色云图标）。
- `sync` 必须关闭：TCP 8443 透传 + mTLS
- `cloud` 建议关闭：避免双重代理干扰 X-Forwarded-For
- `auth` 建议关闭：Lambda Function URL 自带 HTTPS

**Step 2: 验证 DNS 解析**

```bash
dig cloud.你的域名 +short
dig sync.你的域名 +short
dig auth.你的域名 +short
```
Expected: 每个都返回对应的 AWS DNS 名称或 IP

---

### Task 10: 配置 Stripe Webhook

**Step 1: 在 Stripe Dashboard 创建 Webhook Endpoint**

Stripe Dashboard → Developers → Webhooks → Add endpoint:
- URL: `https://cloud.你的域名/stripe-webhook`
- Events:
  - `checkout.session.completed`
  - `customer.subscription.created`
  - `customer.subscription.updated`
  - `customer.subscription.deleted`

**Step 2: 获取 Webhook Signing Secret**

创建后，点击 endpoint → Signing secret → Reveal

**Step 3: 更新 Secrets Manager**

```bash
aws secretsmanager put-secret-value \
  --secret-id crab/production/stripe-webhook-secret \
  --secret-string "whsec_你的签名密钥" \
  --region eu-south-2
```

**Step 4: 重启 ECS 使新 Secret 生效**

```bash
aws ecs update-service \
  --cluster crab-production \
  --service crab-cloud \
  --force-new-deployment \
  --region eu-south-2 > /dev/null
```

---

### Task 11: 冒烟测试

**Step 1: Health Check**

```bash
curl -s https://cloud.你的域名/health
```
Expected: `200 OK`（或 JSON health 响应）

**Step 2: Lambda Health**

```bash
curl -s https://auth.你的域名/
```
Expected: 非 5xx 响应

**Step 3: 检查 ECS 日志**

```bash
aws logs tail /ecs/crab-cloud-production --since 10m --region eu-south-2
```
Expected: 看到 crab-cloud 启动日志，包括 "migrations applied" 类似消息

**Step 4: 部署状态全览**

```bash
./deploy/deploy.sh status
```
Expected: ECS Running=1, Lambda State=Active, RDS Status=available, 无 Active Alarms

**Step 5: 注册测试租户**

```bash
curl -s -X POST https://cloud.你的域名/register \
  -H "Content-Type: application/json" \
  -d '{"email":"你验证过的邮箱","password":"TestPassword123!","restaurant_name":"测试餐厅"}' | jq .
```
Expected: 注册成功响应

**Step 6: 检查验证邮件**

查看邮箱是否收到验证邮件。如果在 SES 沙箱模式，确保收件人已验证。

**Step 7: Stripe Webhook 测试**

```bash
stripe listen --forward-to https://cloud.你的域名/stripe-webhook
```
在另一个终端：
```bash
stripe trigger checkout.session.completed
```
Expected: Webhook 正确处理，无 4xx/5xx 错误

---

### Task 12: SNS 告警邮件确认

CloudFormation 创建了 SNS subscription，AWS 会发确认邮件。

**Step 1: 查看邮箱**

检查 `ALERT_EMAIL` 邮箱，找到 AWS SNS 确认邮件。

**Step 2: 点击确认链接**

点击邮件中的 "Confirm subscription" 链接。

**Step 3: 验证**

```bash
aws sns list-subscriptions-by-topic \
  --topic-arn $(aws cloudformation describe-stacks --stack-name crab-production --query 'Stacks[0].Outputs[?OutputKey==`AlarmTopicArn`].OutputValue' --output text --region eu-south-2) \
  --region eu-south-2 \
  --query 'Subscriptions[].{Endpoint:Endpoint,Status:SubscriptionArn}'
```
Expected: Status 不是 `PendingConfirmation`

---

## 执行顺序和依赖

```
Task 1 (ECR fix) ──────────────────────┐
Task 2 (OIDC template) ────────────────┤
Task 3 (ECS Exec) ─────────────────────┤── 代码改动，可并行
                                        │
                                        ▼ commit + push
Task 4 (部署 OIDC 栈) ◄── Task 2
Task 5 (ACM 证书) ─────────── 可与 Task 4 并行
Task 6 (SES 配置) ─────────── 可与 Task 4/5 并行
                                        │
                                        ▼ 等 ACM 证书验证通过
Task 7 (部署主栈) ◄── Task 1, 3, 5
                                        │
                                        ▼
Task 8 (配置 Secrets) ◄── Task 7
Task 9 (DNS 配置) ◄── Task 7
Task 10 (Stripe Webhook) ◄── Task 9
                                        │
                                        ▼
Task 11 (冒烟测试) ◄── Task 8, 9, 10
Task 12 (SNS 确认) ◄── Task 7
```

## 注意事项

1. **数据库迁移不需要单独执行** — `crab-cloud` 在 `state.rs:127` 启动时自动运行 `sqlx::migrate!()`
2. **crab-auth 的迁移** — Lambda 不使用 sqlx migrations，它的表结构由 crab-cloud 的迁移管理（共享同一个 PostgreSQL）
3. **Task 1-3 是代码改动**，可以一次 commit 或分别 commit
4. **Task 4-12 是运维操作**，需要在 AWS 控制台 / CLI 执行
5. **内测阶段 SES 沙箱限制** — 只能给已验证邮箱发信，足够内测使用
