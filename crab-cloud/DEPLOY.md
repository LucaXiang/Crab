# crab-cloud 部署指南

## 架构

EC2 + Docker Compose + Caddy (自动 HTTPS)

```
Internet
    │
    ├─ 80/443 → Caddy (自动 TLS, Let's Encrypt)
    │              └─→ crab-cloud:8080 (HTTP API)
    │
    └─ 8443 (直通) → crab-cloud:8443 (mTLS, 边缘同步)
```

域名:
- `cloud.redcoral.app` → crab-cloud (HTTP API)
- `auth.redcoral.app` → crab-cloud (HTTP API, 同一服务)

## 你需要准备的

### 1. 域名 + DNS (Cloudflare)

- [ ] `cloud.redcoral.app` → A 记录 → EC2 Elastic IP
- [ ] `auth.redcoral.app` → A 记录 → EC2 Elastic IP (同一 IP)
- [ ] Cloudflare Proxy 关闭 (DNS Only)，由 Caddy 管理 TLS

### 2. AWS 资源

- [ ] EC2 实例 (Amazon Linux 2023, t3.micro/small)
- [ ] Elastic IP
- [ ] ECR 仓库 (`crab-cloud`)
- [ ] Security Group: 开放 80, 443, 8443, 22
- [ ] IAM Instance Profile: ECR pull 权限
- [ ] SES 验证域名 + 申请 Production Access

### 3. Stripe

- [ ] `STRIPE_SECRET_KEY` (sk_live_...)
- [ ] Webhook: `https://cloud.redcoral.app/stripe/webhook`
- [ ] `STRIPE_WEBHOOK_SECRET` (whsec_...)

## EC2 目录结构

```
/opt/crab/
├── docker-compose.yml
├── Caddyfile
├── .env                 # 敏感配置
├── certs/               # mTLS 证书
│   ├── root_ca.pem
│   ├── server.pem
│   └── server.key
└── data/                # (预留)
```

## 首次部署

```bash
# 1. 运行 setup.sh 安装 Docker + Docker Compose + ECR 登录
scp -i deploy/ec2/crab-ec2.pem deploy/ec2/setup.sh ec2-user@<IP>:/tmp/
ssh -i deploy/ec2/crab-ec2.pem ec2-user@<IP> "bash /tmp/setup.sh"

# 2. 上传配置文件
scp -i deploy/ec2/crab-ec2.pem deploy/ec2/docker-compose.yml ec2-user@<IP>:/opt/crab/
scp -i deploy/ec2/crab-ec2.pem deploy/ec2/Caddyfile ec2-user@<IP>:/opt/crab/

# 3. 创建 .env (从 .env.example 修改)
scp -i deploy/ec2/crab-ec2.pem deploy/ec2/.env ec2-user@<IP>:/opt/crab/

# 4. 上传 mTLS 证书
scp -i deploy/ec2/crab-ec2.pem certs/* ec2-user@<IP>:/opt/crab/certs/

# 5. 启动
ssh -i deploy/ec2/crab-ec2.pem ec2-user@<IP> "cd /opt/crab && docker-compose up -d"

# 6. 验证
curl https://cloud.redcoral.app/health
```

## 日常部署

```bash
# 本地构建推送
docker build -t crab-cloud -f crab-cloud/Dockerfile .
docker tag crab-cloud:latest 364453382269.dkr.ecr.eu-south-2.amazonaws.com/crab-cloud:latest
aws ecr get-login-password --region eu-south-2 | docker login --username AWS --password-stdin 364453382269.dkr.ecr.eu-south-2.amazonaws.com
docker push 364453382269.dkr.ecr.eu-south-2.amazonaws.com/crab-cloud:latest

# EC2 上拉取重启
ssh -i deploy/ec2/crab-ec2.pem ec2-user@<IP> \
  "cd /opt/crab && \
   aws ecr get-login-password --region eu-south-2 | docker login --username AWS --password-stdin 364453382269.dkr.ecr.eu-south-2.amazonaws.com && \
   docker-compose pull crab-cloud && \
   docker-compose up -d crab-cloud"

# 查看日志
ssh -i deploy/ec2/crab-ec2.pem ec2-user@<IP> "cd /opt/crab && docker-compose logs -f crab-cloud --tail 50"
```

## 安全措施

| 措施 | 说明 |
|------|------|
| **Caddy 自动 TLS** | Let's Encrypt 证书自动签发/续期 |
| **mTLS 8443** | Entity Cert (Tenant CA 签发) 双向认证 |
| **PostgreSQL** | 仅 localhost 暴露，Docker 网络内部访问 |
| **Security Group** | 仅开放 80/443/8443/22 |
| **.env 文件** | 敏感配置不进 Git |

## 月费估算

| 组件 | 月费 |
|------|------|
| EC2 t3.small | ~$15 |
| Elastic IP | $0 (绑定运行中实例) |
| ECR | ~$0 |
| SES | ~$0 |
| **总计** | **~$15/月** |

## 推荐区域

`eu-south-2`（西班牙，马德里）— 最低延迟，数据驻留合规。
