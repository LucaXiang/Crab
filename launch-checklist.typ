#set document(title: "Crab SaaS 内测上线操作手册", author: "xzy")
#set page(paper: "a4", margin: (x: 2cm, y: 2.5cm), numbering: "1")
#set text(font: ("PingFang SC", "Heiti SC"), size: 10pt, lang: "zh")
#set heading(numbering: (..nums) => {
  let n = nums.pos()
  if n.len() == 1 {
    "第 " + str(n.at(0)) + " 步 "
  } else if n.len() == 2 {
    str(n.at(0)) + "." + str(n.at(1)) + " "
  } else {
    ""
  }
})
#show heading.where(level: 1): it => {
  v(1em)
  block(
    fill: rgb("#2c3e50"),
    inset: (x: 12pt, y: 8pt),
    radius: 4pt,
    width: 100%,
    text(size: 13pt, weight: "bold", fill: white, it)
  )
  v(0.4em)
}
#show heading.where(level: 2): it => {
  v(0.5em)
  text(size: 11pt, weight: "bold", fill: rgb("#2c3e50"), it)
  v(0.3em)
}
#show raw.where(block: true): it => {
  block(fill: rgb("#f8f9fa"), inset: 10pt, radius: 4pt, width: 100%, it)
}

#let tip(body) = {
  block(
    fill: rgb("#e8f5e9"),
    inset: 10pt,
    radius: 4pt,
    width: 100%,
    [#text(weight: "bold", fill: rgb("#2e7d32"))[TIP] #body]
  )
}

#let warn(body) = {
  block(
    fill: rgb("#fff3e0"),
    inset: 10pt,
    radius: 4pt,
    width: 100%,
    [#text(weight: "bold", fill: rgb("#e65100"))[注意] #body]
  )
}

#let check(body) = {
  [#text(fill: rgb("#27ae60"))[✓] #body]
}

#let todo(body) = {
  [#text(fill: rgb("#e74c3c"))[☐] #body]
}

// ─── Title ───
#align(center)[
  #v(3cm)
  #text(size: 28pt, weight: "bold", fill: rgb("#2c3e50"))[Crab SaaS]
  #v(0.3cm)
  #text(size: 16pt, fill: rgb("#7f8c8d"))[内测上线操作手册]
  #v(1cm)
  #line(length: 60%, stroke: 0.5pt + rgb("#bdc3c7"))
  #v(0.5cm)
  #text(size: 10pt, fill: rgb("#95a5a6"))[
    AWS eu-south-2 · Cloudflare DNS · GitHub Actions CI/CD \
    版本: 2026-02-18 · 预计执行时间: 1-2 小时
  ]
  #v(3cm)
]

// ─── Overview ───
#text(size: 14pt, weight: "bold", fill: rgb("#2c3e50"))[执行概览]
#v(0.5em)

本手册包含 9 个步骤，将 crab-auth (Lambda) 和 crab-cloud (ECS Fargate) 部署到 AWS 生产环境。

#block(fill: rgb("#f8f9fa"), inset: 15pt, radius: 6pt, width: 100%)[
  #set text(size: 9pt, font: "Menlo")
  ```
  步骤 1: OIDC 栈 ──┐
  步骤 2: ACM 证书 ──┼── 可并行（约 10 分钟）
  步骤 3: SES 邮件 ──┘
                      │
                      ▼ 等 ACM 验证通过
  步骤 4: 部署主栈 ◄───── 核心步骤（15-25 分钟）
                      │
                      ▼
  步骤 5: 配置 Secrets ──┐
  步骤 6: DNS 配置 ──────┼── 可并行
  步骤 7: Stripe Webhook ┘
                      │
                      ▼
  步骤 8: 冒烟测试
  步骤 9: 告警确认
  ```
]

#v(0.5em)

#text(size: 14pt, weight: "bold", fill: rgb("#2c3e50"))[准备清单]
#v(0.5em)

#table(
  columns: (auto, 1fr, auto),
  inset: 8pt,
  align: (center, left, center),
  fill: (_, y) => if y == 0 { rgb("#2c3e50") } else if calc.odd(y) { rgb("#f8f9fa") } else { white },
  table.header(
    text(fill: white, weight: "bold")[项目],
    text(fill: white, weight: "bold")[说明],
    text(fill: white, weight: "bold")[状态],
  ),
  [AWS CLI], [已安装且配置 eu-south-2 区域], [☐],
  [AWS 账号], [有 IAM 管理权限], [☐],
  [域名], [Cloudflare DNS 托管], [☐],
  [Stripe], [已有账号，有 API Key], [☐],
  [GitHub], [repo 有 Settings 权限], [☐],
  [代码], [已 push 最新 commit (含 OIDC 模板)], [☐],
)

#pagebreak()

// ═══════════════════════════════════════════════════
= 部署 GitHub OIDC 栈

让 GitHub Actions 通过 OIDC 免密认证 AWS，不需要存储 Access Key。

== 创建 Lambda S3 Bucket

```bash
aws s3 mb s3://crab-deploy-artifacts --region eu-south-2
```

#check[输出: `make_bucket: crab-deploy-artifacts`]

== 部署 OIDC CloudFormation 栈

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

等待完成（约 2 分钟）。

== 获取 Role ARN

```bash
aws cloudformation describe-stacks \
  --stack-name crab-github-oidc \
  --query 'Stacks[0].Outputs[?OutputKey==`DeployRoleArn`].OutputValue' \
  --output text \
  --region eu-south-2
```

#check[记录输出的 ARN: `arn:aws:iam::XXXX:role/crab-github-deploy`]

== 设置 GitHub Secrets

打开 GitHub repo → *Settings* → *Secrets and variables* → *Actions*，添加：

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#2c3e50") } else if calc.odd(y) { rgb("#f8f9fa") } else { white },
  table.header(
    text(fill: white, weight: "bold")[Secret Name],
    text(fill: white, weight: "bold")[值],
  ),
  [`AWS_DEPLOY_ROLE_ARN`], [上一步获取的 Role ARN],
  [`LAMBDA_S3_BUCKET`], [`crab-deploy-artifacts`],
)

#tip[其他 Secrets（TAURI 签名、UPDATE_S3 等）内测阶段暂不需要。]

#pagebreak()

// ═══════════════════════════════════════════════════
= ACM 证书 (HTTPS)

#warn[此步骤可与步骤 1、3 *并行*执行。]

== 申请通配符证书

```bash
aws acm request-certificate \
  --domain-name "你的域名" \
  --subject-alternative-names "*.你的域名" \
  --validation-method DNS \
  --region eu-south-2
```

#check[记录输出的 `CertificateArn`，步骤 4 需要。]

== 获取 DNS 验证记录

```bash
aws acm describe-certificate \
  --certificate-arn 上一步的ARN \
  --query 'Certificate.DomainValidationOptions[].ResourceRecord' \
  --output table \
  --region eu-south-2
```

输出一条 CNAME 记录 (Name + Value)。

== 在 Cloudflare 添加验证 CNAME

Cloudflare Dashboard → 你的域名 → *DNS* → *Add record*:

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#ecf0f1") } else { white },
  [*Type*], [CNAME],
  [*Name*], [ACM 给的 Name（去掉你的域名后缀部分）],
  [*Target*], [ACM 给的 Value],
  [*Proxy*], [❌ DNS only（灰色云）],
)

== 等待验证通过

```bash
aws acm wait certificate-validated \
  --certificate-arn 你的ARN \
  --region eu-south-2
```

命令无输出即成功。通常需要 5-10 分钟。

== 验证

```bash
aws acm describe-certificate \
  --certificate-arn 你的ARN \
  --region eu-south-2 \
  --query 'Certificate.Status'
# 输出: "ISSUED"
```

#warn[证书必须在 `eu-south-2` 区域申请，与部署区域一致。]

#pagebreak()

// ═══════════════════════════════════════════════════
= SES 邮件服务

用于发送注册验证邮件。

#warn[此步骤可与步骤 1、2 *并行*执行。]

== 验证域名

```bash
aws sesv2 create-email-identity \
  --email-identity 你的域名 \
  --region eu-south-2
```

== 获取 DKIM 记录

```bash
aws sesv2 get-email-identity \
  --email-identity 你的域名 \
  --query 'DkimAttributes.Tokens' \
  --output text \
  --region eu-south-2
```

输出 3 个 token（空格分隔）。

== 在 Cloudflare 添加 DKIM CNAME

对输出的每个 token，在 Cloudflare 添加一条 CNAME：

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#2c3e50") } else if calc.odd(y) { rgb("#f8f9fa") } else { white },
  table.header(
    text(fill: white, weight: "bold")[字段],
    text(fill: white, weight: "bold")[值],
  ),
  [*Type*], [CNAME],
  [*Name*], [`{token}._domainkey`],
  [*Target*], [`{token}.dkim.amazonses.com`],
  [*Proxy*], [❌ DNS only],
)

重复 3 次，每个 token 一条。

== (可选) 添加 SPF 记录

在 Cloudflare 添加 TXT 记录：
- *Name*: `@`
- *Value*: `v=spf1 include:amazonses.com ~all`

== 内测阶段：验证收件人邮箱

SES 沙箱模式只能给已验证邮箱发信。验证你自己的邮箱：

```bash
aws sesv2 create-email-identity \
  --email-identity your-email@example.com \
  --region eu-south-2
```

去邮箱点击验证链接。

#tip[内测阶段不需要申请移出沙箱。只要把测试用的收件人邮箱都验证一下就行。]

#pagebreak()

// ═══════════════════════════════════════════════════
= 部署主 CloudFormation 栈

#block(
  fill: rgb("#fff3e0"),
  inset: 12pt,
  radius: 4pt,
  width: 100%,
)[
  #text(weight: "bold", fill: rgb("#e65100"), size: 12pt)[这是核心步骤]

  一条命令创建所有 AWS 基础设施（约 50 个资源，耗时 15-25 分钟）。
  确保步骤 2 的 ACM 证书已 `ISSUED`。
]

== 执行部署

```bash
ACM_CERTIFICATE_ARN="arn:aws:acm:eu-south-2:xxx:certificate/xxx" \
ALERT_EMAIL="你的告警邮箱" \
./deploy/deploy.sh setup
```

这会自动执行：
+ 构建 crab-auth Lambda zip（Docker 交叉编译 aarch64）
+ 上传 Lambda zip 到 S3
+ 构建 crab-cloud Docker 镜像
+ 推送到 ECR
+ 部署 CloudFormation 栈

== 创建的资源

#table(
  columns: (auto, 1fr, auto),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#2c3e50") } else if calc.odd(y) { rgb("#f8f9fa") } else { white },
  table.header(
    text(fill: white, weight: "bold")[资源],
    text(fill: white, weight: "bold")[说明],
    text(fill: white, weight: "bold")[月费],
  ),
  [VPC], [2 公有 + 2 私有子网, NAT Gateway], [~\$35],
  [RDS], [PostgreSQL 16, db.t4g.micro, 加密, 14 天备份], [~\$15],
  [ECS Fargate], [0.25 vCPU, 512MB (crab-cloud)], [~\$10],
  [ALB], [HTTPS 443, TLS 1.3, WAF 关联], [~\$18],
  [NLB], [TCP 8443, mTLS 透传], [~\$18],
  [Lambda], [arm64, 256MB (crab-auth)], [~\$1],
  [WAF v2], [限速 1000 req/5min + AWS 托管规则], [含在 ALB],
  [ECR + S3], [镜像仓库 + 证书桶 (KMS 加密)], [~\$1],
  [Secrets Manager], [4 个密钥], [~\$1],
  [CloudWatch], [9 个告警 → SNS 邮件通知], [\$0],
  [*合计*], [], [*~\$99/月*],
)

== 记录 Stack Outputs

部署完成后会输出关键信息。*务必记录*：

- #todo[*ALBDnsName*: `crab-xxx.eu-south-2.elb.amazonaws.com`]
- #todo[*NLBDnsName*: `crab-mtls-xxx.elb.eu-south-2.amazonaws.com`]
- #todo[*CrabAuthFunctionUrl*: `https://xxx.lambda-url.eu-south-2.on.aws/`]
- #todo[*RDSEndpoint*: `crab-production.xxx.eu-south-2.rds.amazonaws.com`]

如果忘了记录，可以再查：

```bash
aws cloudformation describe-stacks \
  --stack-name crab-production \
  --query 'Stacks[0].Outputs' \
  --output table \
  --region eu-south-2
```

#pagebreak()

// ═══════════════════════════════════════════════════
= 配置 Secrets

== 获取 RDS 密码

CloudFormation 自动管理 RDS 密码。找到它：

```bash
# 找到 RDS 管理的 Secret 名称
aws secretsmanager list-secrets \
  --filter Key=name,Values=rds \
  --query 'SecretList[].Name' \
  --output text \
  --region eu-south-2
```

```bash
# 获取密码
aws secretsmanager get-secret-value \
  --secret-id 上面找到的secret名 \
  --query 'SecretString' \
  --output text \
  --region eu-south-2 | python3 -c "
import sys, json
d = json.load(sys.stdin)
print(f'postgres://{d[\"username\"]}:{d[\"password\"]}@RDS端点:5432/crab')
"
```

把输出中的 `RDS端点` 替换为步骤 4 记录的 *RDSEndpoint*。

== 生成 JWT Secret

```bash
openssl rand -hex 32
```

#check[记录输出的 64 位十六进制字符串。]

== 运行 Secrets 脚本

```bash
./deploy/deploy.sh secrets
```

交互式输入 4 个值：

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#2c3e50") } else if calc.odd(y) { rgb("#f8f9fa") } else { white },
  table.header(
    text(fill: white, weight: "bold")[变量],
    text(fill: white, weight: "bold")[值],
  ),
  [`database-url`], [`postgres://crab:密码@RDS端点:5432/crab`],
  [`stripe-secret-key`], [`sk_test_...`（内测用测试密钥）],
  [`stripe-webhook-secret`], [先输入占位值如 `placeholder`，步骤 7 再更新],
  [`jwt-secret`], [上面 `openssl rand` 生成的值],
)

== 重启 ECS 使 Secrets 生效

```bash
aws ecs update-service \
  --cluster crab-production \
  --service crab-cloud \
  --force-new-deployment \
  --region eu-south-2 > /dev/null

echo "等待服务稳定..."
aws ecs wait services-stable \
  --cluster crab-production \
  --services crab-cloud \
  --region eu-south-2

echo "✓ 服务已就绪"
```

#warn[crab-cloud 启动时会自动运行 PostgreSQL 迁移。如果 DATABASE\_URL 正确，数据库 schema 会自动创建。]

#pagebreak()

// ═══════════════════════════════════════════════════
= Cloudflare DNS 配置

用步骤 4 记录的 Stack Outputs 值。

== 添加 3 条 CNAME 记录

在 Cloudflare Dashboard → 你的域名 → *DNS* → *Add record*：

#table(
  columns: (auto, auto, 1fr, auto),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#2c3e50") } else if calc.odd(y) { rgb("#f8f9fa") } else { white },
  table.header(
    text(fill: white, weight: "bold")[Type],
    text(fill: white, weight: "bold")[Name],
    text(fill: white, weight: "bold")[Target],
    text(fill: white, weight: "bold")[Proxy],
  ),
  [CNAME], [`cloud`], [步骤 4 的 ALBDnsName], [❌ off],
  [CNAME], [`sync`], [步骤 4 的 NLBDnsName], [❌ off],
  [CNAME], [`auth`], [步骤 4 的 CrabAuthFunctionUrl (去掉 `https://` 和尾部 `/`)], [❌ off],
)

#v(0.5em)

#block(
  fill: rgb("#ffebee"),
  inset: 10pt,
  radius: 4pt,
  width: 100%,
)[
  #text(weight: "bold", fill: rgb("#c62828"))[所有记录的 Proxy 必须关闭（灰色云图标）！]

  - *`sync`*: #text(weight: "bold")[必须]关闭 — NLB TCP 透传 + mTLS，Cloudflare 代理会中断 TLS 握手
  - *`cloud`*: 建议关闭 — ALB 已有 WAF + HTTPS，双重代理增加延迟、干扰 X-Forwarded-For
  - *`auth`*: 建议关闭 — Lambda Function URL 自带 HTTPS
]

== 验证 DNS 解析

```bash
dig cloud.你的域名 +short
dig sync.你的域名 +short
dig auth.你的域名 +short
```

每个都应该返回对应的 AWS DNS 名称或 IP 地址。

#tip[DNS 传播通常很快（Cloudflare 几秒钟），但偶尔需要等几分钟。]

#pagebreak()

// ═══════════════════════════════════════════════════
= Stripe Webhook 配置

== 创建 Webhook Endpoint

Stripe Dashboard → *Developers* → *Webhooks* → *Add endpoint*:

#table(
  columns: (auto, 1fr),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#ecf0f1") } else { white },
  [*Endpoint URL*], [`https://cloud.你的域名/stripe-webhook`],
  [*Version*], [Latest API version],
)

订阅以下事件：

#table(
  columns: (1fr,),
  inset: 6pt,
  fill: (_, y) => if calc.odd(y) { rgb("#f8f9fa") } else { white },
  [`checkout.session.completed`],
  [`customer.subscription.created`],
  [`customer.subscription.updated`],
  [`customer.subscription.deleted`],
  [`invoice.payment_failed`],
)

== 获取 Signing Secret

创建完成后，点击该 endpoint → *Signing secret* → *Reveal*。

记录 `whsec_...` 值。

== 更新 Secrets Manager

```bash
aws secretsmanager put-secret-value \
  --secret-id "crab/production/stripe-webhook-secret" \
  --secret-string "whsec_你的签名密钥" \
  --region eu-south-2
```

== 重启 ECS 使新 Secret 生效

```bash
aws ecs update-service \
  --cluster crab-production \
  --service crab-cloud \
  --force-new-deployment \
  --region eu-south-2 > /dev/null
```

#tip[ECS 新 Task 启动时会拉取最新的 Secrets Manager 值，旧 Task 会被自动替换。]

#pagebreak()

// ═══════════════════════════════════════════════════
= 冒烟测试

依次执行以下测试，确认所有服务正常。

== Health Check

```bash
curl -s https://cloud.你的域名/health
```

#check[期望: HTTP 200，返回健康状态]

== Lambda 响应

```bash
curl -s https://auth.你的域名/
```

#check[期望: 非 5xx 响应]

== ECS 日志检查

```bash
aws logs tail /ecs/crab-cloud-production \
  --since 10m \
  --region eu-south-2
```

#check[期望: 看到启动日志，含 migration 相关信息]

== 部署状态全览

```bash
./deploy/deploy.sh status
```

#check[ECS: Running = 1, Desired = 1]
#check[Lambda: State = Active]
#check[RDS: Status = available]
#check[Active Alarms: None]

== 注册测试租户

```bash
curl -s -X POST https://cloud.你的域名/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "你验证过的邮箱",
    "password": "TestPassword123!",
    "restaurant_name": "测试餐厅"
  }' | python3 -m json.tool
```

#check[期望: 返回注册成功的 JSON 响应]

== 验证邮件

检查邮箱是否收到验证邮件。

#warn[SES 沙箱模式下，收件人邮箱必须在步骤 3 中已验证。]

== Stripe Webhook 测试

安装 Stripe CLI（如未安装）:

```bash
brew install stripe/stripe-cli/stripe
stripe login
```

监听并转发：

```bash
stripe listen --forward-to https://cloud.你的域名/stripe-webhook
```

在另一个终端触发测试事件：

```bash
stripe trigger checkout.session.completed
```

#check[期望: stripe listen 输出显示事件已成功处理 (200)]

#pagebreak()

// ═══════════════════════════════════════════════════
= SNS 告警确认

CloudFormation 创建了 SNS 订阅，AWS 会发送确认邮件到你的告警邮箱。

== 确认订阅

+ 打开步骤 4 中 `ALERT_EMAIL` 指定的邮箱
+ 找到来自 AWS 的确认邮件
+ 点击 *Confirm subscription* 链接

== 验证

```bash
TOPIC_ARN=$(aws cloudformation describe-stacks \
  --stack-name crab-production \
  --query 'Stacks[0].Outputs[?OutputKey==`AlarmTopicArn`].OutputValue' \
  --output text \
  --region eu-south-2)

aws sns list-subscriptions-by-topic \
  --topic-arn "$TOPIC_ARN" \
  --query 'Subscriptions[].{Endpoint:Endpoint,Status:SubscriptionArn}' \
  --output table \
  --region eu-south-2
```

#check[期望: Status 不是 `PendingConfirmation`，而是一个完整的 ARN]

#pagebreak()

// ═══════════════════════════════════════════════════
#text(size: 14pt, weight: "bold", fill: rgb("#2c3e50"))[完成检查清单]
#v(0.5em)

所有步骤完成后，逐项确认：

#table(
  columns: (auto, 1fr, auto),
  inset: 8pt,
  fill: (_, y) => if y == 0 { rgb("#2c3e50") } else if calc.odd(y) { rgb("#f8f9fa") } else { white },
  table.header(
    text(fill: white, weight: "bold")[步骤],
    text(fill: white, weight: "bold")[检查项],
    text(fill: white, weight: "bold")[状态],
  ),
  [1], [GitHub OIDC Role 已创建，GitHub Secrets 已配置], [☐],
  [2], [ACM 证书状态 = ISSUED], [☐],
  [3], [SES 域名已验证，收件人邮箱已验证], [☐],
  [4], [CloudFormation 栈状态 = CREATE\_COMPLETE], [☐],
  [5], [Secrets Manager 4 个密钥都有真实值], [☐],
  [6], [3 条 DNS CNAME 已添加，Proxy 全部关闭], [☐],
  [7], [Stripe Webhook endpoint 已创建，Secret 已更新], [☐],
  [8], [Health check 返回 200，注册流程正常], [☐],
  [9], [SNS 告警邮件已确认], [☐],
)

#v(1em)

#block(
  fill: rgb("#e8f5e9"),
  inset: 14pt,
  radius: 4pt,
  width: 100%,
)[
  #text(weight: "bold", fill: rgb("#2e7d32"), size: 12pt)[恭喜！内测环境已就绪。]

  #v(0.3em)

  现在可以：
  - 向 main 分支 push 代码，CI/CD 自动构建部署
  - 注册测试租户，验证完整流程
  - 在餐厅安装 edge-server + POS 客户端，测试激活和同步

  #v(0.3em)

  *后续待办（非紧急）：*
  - 申请 SES Production Access（移出沙箱）
  - 配置 Tauri 签名密钥 + Release 流水线
  - 创建 CloudWatch Dashboard
  - 编写运维 Runbook
]

#v(2em)
#align(center)[
  #text(size: 9pt, fill: rgb("#95a5a6"))[
    Crab SaaS 内测上线操作手册 · 2026-02-18 · eu-south-2 (西班牙)
  ]
]
