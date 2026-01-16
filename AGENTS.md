# CRAB PROJECT KNOWLEDGE BASE

**Generated:** 2026-01-16T18:07:10Z
**Commit:** 2490900
**Branch:** main

## 概览
分布式餐厅管理系统，Rust实现。边缘服务器+客户端架构，mTLS三层证书体系，SurrealDB嵌入式数据库。

## 结构
```
crab/
├── edge-server/    # 边缘服务器核心（8000+行，8模块）
├── shared/         # 协议与类型定义（1500行）
├── crab-client/    # 统一客户端库（2000行）
├── crab-cert/      # PKI证书管理（3000行）
└── crab-auth/      # 认证服务器（轻量）
```

## WHERE TO LOOK
| 任务 | 位置 | 备注 |
|------|------|------|
| 消息总线架构 | edge-server/src/message/ | TCP/TLS/Memory三种传输 |
| 证书管理 | crab-cert/ | 三级CA体系，硬件绑定 |
| 安全配置 | edge-server/src/auth/ | JWT+mTLS |
| 数据库操作 | edge-server/src/db/ | SurrealDB封装 |
| API路由 | edge-server/src/api/ | Axum HTTP服务 |
| 共享协议 | shared/src/message/ | 6种事件类型定义 |

## 编码规范
- **错误处理**: PoC阶段允许unwrap()，生产目标Result<T,E>
- **异步**: Tokio 1.0，#[async_trait]用于trait
- **所有权**: 优先借用，使用Arc共享状态
- **类型系统**: newtypes和traits强制不变量

## 禁止模式
- ❌ 生产环境使用unwrap()：edge-server/src/auth/jwt.rs存在DO NOT USE IN PRODUCTION警告
- ❌ 无TLS启动：tcp_server.rs包含"Do not start TCP server without TLS"限制
- ❌ 跳过panic：.shared/src/message/mod.rs中的panic!宏用于类型验证

## 独特风格
- **Release优化**: lto=true, codegen-units=1, opt-level=3
- **FIPS合规**: aws-lc-rs加密后端
- **模块化传输**: Transport trait可插拔架构
- **Arc状态**: ServerState使用Arc浅拷贝

## 命令
```bash
# 构建
cargo build --workspace
cargo check --workspace

# 测试
cargo test --workspace --lib

# Lint
cargo clippy --workspace -- -D warnings

# 格式化
cargo fmt

# 示例
cargo run -p edge-server --example interactive_demo
cargo run -p crab-client --example message_client
cargo run -p crab-cert --example mtls_demo
```

## 注意事项
- **130+unwrap()调用** - 存在panic风险，需逐步替换
- **消息无持久化** - 崩溃时消息丢失，考虑redb
- **硬件ID可伪造** - 需加强验证机制
- **无Rate Limiting** - 易受暴力破解攻击

## 风险等级
- **P0立即修复**: JWT_SECRET环境变量，panic风险点
- **P1本周完成**: 消息持久化，心跳检测，证书有效期检查
- **P2近期规划**: 日志脱敏，限流中间件，CI/CD配置
