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

## 部署

EC2 + Docker Compose + Caddy (自动 HTTPS)，配置在 `deploy/ec2/`。
- 构建: `./deploy/build-cloud.sh push`
- EC2 路径: `/opt/crab/`
- CI: GitHub Actions → ECR push → SSH 部署到 EC2

## 禁止事项

- 直接删除 Order/OrderEvent 记录 (用 VOID 状态管理)
- 前端直接进行金额浮点运算 (用 `Currency` 类)
- 跳过类型对齐直接部署
- 在非 mTLS 环境传输敏感数据
- 子 crate 单独声明依赖版本
- 使用 String 格式 ID (用 i64)
- 使用 `string` 格式的时间戳 (用 `i64` Unix 毫秒)
- EventApplier 中执行 I/O 或副作用
- 使用 `f64` 进行金额计算 (用 `rust_decimal`)
- 添加转换函数/兼容层/适配器来修复类型不匹配 (从源头修)
- 使用 INTEGER cents 存储金额 (用 REAL + `rust_decimal`)
- 使用 JSON TEXT 列存储嵌套对象 (用独立关联表)

## 修复原则

类型不匹配或数据不一致时，**从 SOURCE 向外修**：数据库 schema → Rust shared 类型 → 前端 TypeScript 类型。禁止反向添加 `Number()`/`String()` 转换包装或适配代码。

## 提交规范

- 提交前必须通过零警告零错误: `cargo clippy --workspace` + `cd red_coral && npx tsc --noEmit`
- 只 stage 当前任务范围内的文件，不包含无关 crate/目录的变更

## 执行风格

- 设计意图明确时直接实现，不要过度提问或扩大范围
- 方向已给出时优先行动，减少规划
- UI 布局指令（按钮位置、网格列数、对齐方式）必须一次到位，实现前逐项核对约束

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
