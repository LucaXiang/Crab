# CLAUDE.md

**现阶段项目是开发阶段, 不要适配层,不要兼容性,不要留下技术债,不要留下历史包裹**

## Crab - 分布式餐饮管理系统

Rust workspace 架构，专注离线优先、边缘计算、mTLS 安全通信的 POS 系统。

## Workspace 成员

| Crate | 用途 | 详细文档 |
|-------|------|----------|
| `shared` | 共享类型、协议、错误系统、事件溯源定义 | [`shared/CLAUDE.md`](shared/CLAUDE.md) |
| `edge-server` | 边缘服务器 (SQLite + Axum + MessageBus + 事件溯源) | [`edge-server/CLAUDE.md`](edge-server/CLAUDE.md) |
| `crab-client` | 统一客户端库 (Local/Remote + Typestate + 心跳重连) | [`crab-client/CLAUDE.md`](crab-client/CLAUDE.md) |
| `crab-cloud` | 云端统一服务 (租户 + PKI + 订阅 + Stripe + 同步) | [`crab-cloud/CLAUDE.md`](crab-cloud/CLAUDE.md) |
| `crab-cert` | PKI/证书管理 (Root CA → Tenant CA → Entity) | [`crab-cert/CLAUDE.md`](crab-cert/CLAUDE.md) |
| `crab-printer` | ESC/POS 热敏打印底层库 (GBK 编码) | [`crab-printer/CLAUDE.md`](crab-printer/CLAUDE.md) |
| `red_coral` | **Tauri POS 前端** (React 19 + Zustand + Tailwind) | [`red_coral/CLAUDE.md`](red_coral/CLAUDE.md) |

## 命令

```bash
# Rust workspace
cargo check --workspace        # 类型检查
cargo build --workspace        # 编译
cargo test --workspace --lib   # 测试
cargo clippy --workspace       # Lint

# SQLx CLI (详见 memory/sqlx-cli-skill.md)
sqlx migrate add -r -s <desc> --source edge-server/migrations   # 新建迁移
sqlx migrate run --source edge-server/migrations                 # 运行迁移
sqlx migrate info --source edge-server/migrations                # 查看状态
sqlx db reset -y --source edge-server/migrations                 # 重置数据库
cargo sqlx prepare --workspace                                   # 离线元数据

# POS 前端 (red_coral/)
cd red_coral && npm run tauri:dev   # Tauri 开发
cd red_coral && npx tsc --noEmit    # TS 类型检查
```

## 核心架构

- **edge-server**: 餐厅本地运行，内含 Axum API + MessageBus (TCP/TLS) + SQLite + redb 事件存储
- **red_coral**: Tauri POS 前端，双模式运行 — Server 模式内嵌 edge-server (LocalClient)，Client 模式 mTLS 远程连接 (RemoteClient)
- **crab-cloud**: 云端统一服务 (租户管理 + PKI/认证 + 订阅校验 + Stripe + 数据同步)，EC2 + Docker Compose + Caddy 部署
- **订单系统**: Event Sourcing + CQRS，redb 存储事件，SQLite 归档查询
- **发票系统 (Verifactu)**: 西班牙 AEAT 电子发票合规，Edge 本地生成 → Cloud 同步 → AEAT 提交

### 订单层架构（Order Layer）

**四层结构**：

```
redb (活跃订单)                    ← Event Sourcing 运行时
  │ 完成/作废/合并
  ▼
SQLite archived_order              ← 归档层 (只读业务数据)
  + archived_order_event           ← 事件级 hash 链 (prev_hash/curr_hash)
  + credit_note / credit_note_item ← 追加式退款凭证
  + invoice                        ← Verifactu 发票 (F2 销售 / R5 更正)
  │
chain_entry (统一 hash 链索引)     ← 唯一的链真相，ORDER + CREDIT_NOTE 交错
  │ CloudWorker 按 chain_entry.id 顺序同步
  ▼
crab-cloud PostgreSQL              ← 云端存储 + hash 再验证 + AEAT 状态跟踪
  + store_invoices                 ← 发票同步 (huella 验证后入库)
```

**hash chain 设计**：
- `chain_entry` 是唯一的 hash 链，ORDER 和 CREDIT_NOTE 共享同一条链
- `system_state.last_chain_hash` 是链尾指针，每次追加在同一事务内原子更新
- `hash_chain_lock` (Mutex) 序列化所有链写入，防 TOCTOU
- Hash 计算: `shared::order::canonical` 提供确定性二进制序列化 (`CanonicalHash` trait)，与 serde 解耦
- `write_f64` 规范化 `-0.0` → `0.0`，保证 JSON roundtrip 稳定性
- 云端反序列化后通过 `verify_hash()` 重新计算 hash 对比，不匹配则 warn（不拒绝）

**发票 huella 链 (Verifactu)**：
- 发票独立于 chain_entry，有自己的 huella 链 (`system_state.last_huella`)
- Huella 按 AEAT 规范: SHA-256 of `"IDEmisorFactura=NIF&NumSerieFactura=NUM&FechaExpedicionFactura=DATE&TipoFactura=TYPE&CuotaTotal=TAX&ImporteTotal=TOTAL&Huella=PREV&FechaHoraHuellaRegistro=TIMESTAMP"`
- `InvoiceService` 在归档/退款时自动创建 F2/R5 发票
- Cloud 端 `InvoiceSync::verify_huella()` 验证 huella 一致性后才入库

**编号体系**：
- `receipt_number`: `FAC{YYYYMMDD}{10000+N}`，全局单调递增计数器 (`system_state.order_count`)
- `credit_note_number`: `CN-{YYYYMMDD}-{NNNN}`，按日计数
- `invoice_number`: `{Serie}{YYYYMMDD}{NNNN}`，每终端独立 Serie，本地分配

**AEAT 状态流转**：
- `PENDING` → Cloud 接收 → `SUBMITTED` → AEAT 响应 → `ACCEPTED` / `REJECTED`
- Cloud→Edge 通过 `StoreOp::UpdateInvoiceAeatStatus` WebSocket 回推状态

**关键文件**：
- `shared/src/order/canonical.rs` — hash 计算函数
- `shared/src/models/invoice.rs` — Invoice 模型 + AeatStatus + TipoFactura
- `shared/src/cloud/sync.rs` — InvoiceSync + verify_huella()
- `edge-server/src/archiving/service.rs` — 归档服务 + receipt_number 生成
- `edge-server/src/archiving/credit_note.rs` — 退款凭证服务
- `edge-server/src/archiving/invoice.rs` — 发票创建 (F2/R5)
- `edge-server/src/db/repository/invoice.rs` — 发票 CRUD + 同步查询
- `edge-server/src/cloud/worker.rs` — CloudWorker 同步 (含发票)
- `crab-cloud/src/db/sync_store.rs` — 云端同步入库 + huella 验证

### 架构原则

- **Server/Cloud 是权威**：所有业务逻辑和计算（定价、收据号、税务、发票/huella 等）在 edge-server 或 crab-cloud 完成。Tauri 客户端只做展示，**禁止**在客户端添加计算库（如 rust_decimal）或复制业务逻辑
- **功能独立**：不相关的功能不要合并到同一个 UI 组件（例如厨房小票和标签重打应该是独立 Modal，不是 Tab）
- **订单不可变**：archived_order 和 archived_order_event 永远只读，所有修正通过追加 credit_note 实现
- **防超退**：退款总额不超过原始订单总额，通过 `SUM(credit_note.total_credit)` 实时校验

## 应用数据目录结构

Tauri identifier: `com.craboss.redcoral`

```
~/Library/Application Support/com.craboss.redcoral/   (= app_data_dir)
├── logs/                          # 应用日志
├── config.json                    # AppConfig
└── tenants/                       # 多租户根
    └── {tenant_id}/               # TenantPaths.base
        ├── auth/
        │   └── session.json       # 员工会话缓存
        ├── certs/                 # Client 模式 mTLS 证书
        │   ├── credential.json    # Client 凭证 (CertManager)
        │   ├── entity.pem         # 客户端证书
        │   ├── entity.key.pem     # 客户端私钥
        │   └── tenant_ca.pem     # Tenant CA
        ├── cache/
        │   └── images/            # Client 图片缓存
        └── server/                # Server 模式 = edge-server work_dir
            ├── credential.json    # TenantBinding (Server 凭证)
            ├── certs/
            │   ├── root_ca.pem
            │   ├── tenant_ca.pem
            │   ├── server.pem
            │   └── server.key.pem
            ├── data/
            │   ├── main.db
            │   ├── orders.redb
            │   └── print.redb
            └── images/
```

路径管理:
- `TenantPaths` (`red_coral/src-tauri/src/core/paths.rs`) — Tauri 侧统一路径 API
- `Config` (`edge-server/src/core/config.rs`) — edge-server 侧路径方法 (`database_path()`, `certs_dir()`, `data_dir()` 等)
- 证书扩展名统一使用 `.pem`

## 部署 (crab-cloud → EC2)

**架构**: EC2 (Amazon Linux 2023) + Docker Compose + Caddy (自动 HTTPS) + PostgreSQL 16

**域名**: `cloud.redcoral.app` → Caddy → crab-cloud:8080
**mTLS**: 端口 8443 直接暴露 (edge-server 双向 TLS 连接用)

### 部署安全规则

- **禁止** `docker-compose down && docker-compose up -d` — 这会重启所有服务（包括生产！）
- **必须** 指定服务名: `docker-compose up -d dev-cloud` 只重启目标服务
- **prod 和 dev 共用一个 docker-compose**，生产用固定 image tag，dev 用 `:latest`
- **Console 部署必须指定 Vite mode**: `npx vite build --mode development` 用于 dev-console（读 `.env.development` → `dev-cloud.redcoral.app`），`npm run build` 默认 production（读 `.env.production` → `cloud.redcoral.app`）
- **不要混淆部署目录**: portal → `/opt/crab/portal/`，console → `/opt/crab/console/`，dev-console → `/opt/crab/dev-console/`

### 完整部署流程

```bash
# 1. 本地构建 + 推送到 ECR
./deploy/build-cloud.sh push

# 2. SSH 到 EC2
ssh -i deploy/ec2/crab-ec2.pem ec2-user@51.92.72.162

# 3. ECR 登录 + 拉取新镜像
aws ecr get-login-password --region eu-south-2 | \
  docker login --username AWS --password-stdin 364453382269.dkr.ecr.eu-south-2.amazonaws.com
docker pull 364453382269.dkr.ecr.eu-south-2.amazonaws.com/crab-cloud:latest

# 4. 重启服务 (只重启目标服务！)
cd /opt/crab
docker-compose up -d dev-cloud    # dev 环境
# docker-compose up -d crab-cloud  # 生产环境 (谨慎!)

# 5. 验证
curl https://dev-cloud.redcoral.app/health
# 期望: {"git_hash":"...","service":"crab-cloud","status":"ok","version":"..."}
```

### Console 部署

```bash
# dev-console (访问 dev-cloud)
cd crab-console
npx vite build --mode development    # 关键: --mode development
cp build/index.html build/200.html
scp -i deploy/ec2/crab-ec2.pem -r build/* ec2-user@51.92.72.162:/opt/crab/dev-console/

# production console (访问 cloud.redcoral.app)
cd crab-console
npm run build
cp build/index.html build/200.html
scp -i deploy/ec2/crab-ec2.pem -r build/* ec2-user@51.92.72.162:/opt/crab/console/
```

### 清理数据库 (仅内测阶段)

```bash
# 在 EC2 上
cd /opt/crab
docker-compose down
docker volume rm crab_pgdata    # 删除 PostgreSQL 数据卷
docker-compose up -d            # 重启，自动 migrate
```

### 关键文件

| 文件 | 用途 |
|------|------|
| `deploy/build-cloud.sh` | 构建 Docker 镜像 + 推送 ECR |
| `deploy/ec2/docker-compose.yml` | 生产编排 (Caddy + PG + crab-cloud) |
| `deploy/ec2/Caddyfile` | 反向代理 + 自动 HTTPS |
| `deploy/ec2/.env` | 生产密钥 (不入 git) |
| `deploy/ec2/certs/` | mTLS 证书 (root_ca, server.pem/key) |
| `deploy/ec2/crab-ec2.pem` | SSH 密钥 (不入 git) |

### Portal 部署 (crab-portal → EC2)

```bash
# 在 crab-portal/ 目录下
cd crab-portal
npm run build                    # 构建静态站点到 build/

# 上传到 EC2
scp -i deploy/ec2/crab-ec2.pem -r build/* ec2-user@51.92.72.162:/opt/crab/portal/
```

**关键**: Caddy 容器挂载 `/opt/crab/portal/` → `/srv/portal` (只读)，部署目标是 `/opt/crab/portal/`，**不是** `/srv/portal/`。
**域名**: `redcoral.app` → Caddy file_server → `/srv/portal`
**缓存**: HTML `max-age=3600` (1h), `_app/immutable/*` 永久缓存

### 安全要求

- **全栈 HTTPS**: 所有 auth_url 强制 `https://`，无 `danger_accept_invalid_certs`
- **P12 安全**: 客户电子签名 (P12+密码) 经 AES-256-GCM 加密后存 PG，加密密钥 (MasterKey) 存 AWS Secrets Manager，密码不入日志。上传时自动提取 NIF 并同步到 edge `store_info.nif`
- **mTLS**: edge-server ↔ crab-cloud 通过 8443 端口双向 TLS
- **私钥文件**: 写入私钥/凭据文件必须使用 `crab_cert::write_secret_file()` (Unix 下 0o600 权限)

## 禁止事项

- 直接删除 Order/OrderEvent 记录 (用 VOID 状态管理)
- 前端直接进行金额浮点运算 (用 `Currency` 类)
- 跳过类型对齐直接部署
- 在非 mTLS 环境传输敏感数据
- 子 crate 单独声明依赖版本
- 使用 String 格式 ID (用 i64)
- 使用 `string` 格式的时间戳 (用 `i64` Unix 毫秒)
- EventApplier 中执行 I/O 或副作用
- 使用 `f64` 进行金额**算术运算** (用 `rust_decimal`，存储/传输/序列化用 `f64` 是允许的)

### 金额类型跨层规则

| 层 | 存储 | Rust 查询类型 | JSON 序列化 |
|----|------|-------------|------------|
| **edge-server (SQLite)** | `REAL` | `f64` | `f64` |
| **crab-cloud (PostgreSQL)** | `NUMERIC(12,2)` | `rust_decimal::Decimal` | `f64` (via `serde-with-float`) |
| **前端 (TypeScript)** | — | — | `number` |

- crab-cloud 查询结构体金额字段**必须**用 `Decimal`，在构建 API 响应时通过 `d()` helper 转 `f64`
- `rust_decimal` workspace feature 必须是 `serde-with-float`（序列化为 JSON 数字而非字符串）
- 添加转换函数/兼容层/适配器来修复类型不匹配 (从源头修)
- 使用 INTEGER cents 存储金额 (用 REAL/DOUBLE PRECISION，Rust 侧计算用 `rust_decimal`)
- 使用 JSON TEXT 列存储嵌套对象 (用独立关联表)
- 绕过 `shared::ErrorCode` 自造错误码或字符串错误 (所有错误必须通过 ErrorCode → AppError → ApiResponse 链路)
- 在 crab-cloud (PG) 的查询结构体中用 `f64` 读取 `NUMERIC` 列 (sqlx 无法解码 PG NUMERIC → f64，必须用 `rust_decimal::Decimal`，在响应边界转 f64)
- 用 `::float8` SQL cast 绕过类型不匹配 (属于打补丁，应从 Rust struct 类型修)

## 修复原则

类型不匹配或数据不一致时，**从 SOURCE 向外修**：数据库 schema → Rust shared 类型 → 前端 TypeScript 类型。禁止反向添加 `Number()`/`String()` 转换包装或适配代码。

## 错误处理规范

**全栈统一错误码**：`shared::ErrorCode` (u16) 是唯一的错误标识，贯穿 Rust → JSON → TypeScript → i18n。

- **Rust 端**: 所有 API 错误必须通过 `AppError` 返回，携带 `ErrorCode`。禁止直接返回裸 `StatusCode` 或自定义错误 JSON
- **JSON 响应**: 统一格式 `{ "code": u16, "message": "...", "data": T?, "details": {}? }`，前端靠 `code` 做 i18n 查表，`message` 仅作 fallback
- **前端 i18n**: 错误码 → `errorCode.<CODE>` 翻译 key，新增错误码时必须同步添加中/西/英翻译
- **新增错误码流程**: `shared/src/error/codes.rs` 添加 variant → `http.rs` 映射 HTTP 状态码 → `TryFrom<u16>` + `message()` + variant count guard 同步更新 → 前端 i18n 翻译
- **Service 层错误**: DB 错误 (`sqlx::Error`) 通过 `From` 自动转为 `AppError(DatabaseError)`，业务错误直接用 `AppError` 便捷构造器 (`not_found()`, `validation()` 等)

## 提交规范

- 提交前必须通过零警告零错误: `cargo clippy --workspace` + `cd red_coral && npx tsc --noEmit`
- **跨 Rust + TypeScript 的变更，两边都必须验证编译**，不要假设一边不受影响
- 只 stage 当前任务范围内的文件，不包含无关 crate/目录的变更
- 先 `git diff --stat` 检查变更范围，用 `git add <specific-files>` 而非 `git add .`

## 版本管理

4 个独立产品，各自独立版本号 (SemVer)：

| 产品 | 说明 | 版本定义位置 |
|------|------|-------------|
| **RedCoral POS** | Tauri 桌面 POS 应用 | 见下方 4 文件 |
| **crab-cloud** | 云端服务 | `Cargo.toml` workspace version |
| **crab-portal** | 官网/落地页 | `crab-portal/package.json` |
| **crab-console** | 管理后台 | `crab-console/package.json` |

### RedCoral POS 版本定义 (4 文件必须同步)

| 文件 | 字段 |
|------|------|
| `Cargo.toml` (workspace root) | `workspace.package.version` |
| `red_coral/src-tauri/Cargo.toml` | `version` |
| `red_coral/src-tauri/tauri.conf.json` | `version` |
| `red_coral/package.json` | `version` |

### Git Hash

- `shared::GIT_HASH` — 编译期自动嵌入 (`shared/build.rs`)
- 所有 health endpoint 返回 `git_hash` 字段
- 无需手动维护

### 发版流程

1. 修改对应产品的版本号文件
2. 提交: `chore(product): bump version to X.Y.Z`
3. 打 tag: `git tag product-vX.Y.Z && git push origin product-vX.Y.Z`
4. RedCoral POS tag 格式 `vX.Y.Z`，触发 CI 自动构建 + S3 上传

## 执行风格

- 设计意图明确时直接实现，不要过度提问或扩大范围
- 方向已给出时优先行动，减少规划
- UI 布局指令（按钮位置、网格列数、对齐方式）必须一次到位，实现前逐项核对约束
- **遇到问题默认做正确的重设计**，不要做增量补丁绕过症状。"fix" = 修根因，不是贴创可贴
- 每个 session 聚焦单一任务，不要主动扩大范围到用户未要求的事情

## 按需加载

处理以下场景时，先读取对应文件：

| 场景 | 文件 |
|------|------|
| 修改跨前后端类型、添加约定 | [`docs/claude/conventions.md`](docs/claude/conventions.md) |
| 编写/审查 tracing 日志 | [`docs/claude/logging.md`](docs/claude/logging.md) |
| 编写测试代码 | [`docs/claude/testing.md`](docs/claude/testing.md) |
| 修改数据库 schema | [`docs/claude/schema-workflow.md`](docs/claude/schema-workflow.md) |

## 响应语言

使用中文回答。
