# EDGE-SERVER MODULE

**Generated:** 2026-01-16T18:07:10Z
**Reason:** 8000+行代码，8子模块，边缘服务架构

## OVERVIEW
轻量级边缘服务器，处理3-4个客户端，mTLS+SurrealDB+消息总线

## STRUCTURE
```
edge-server/src/
├── core/           # 核心状态管理（ServerState, Config）
├── message/        # 消息总线（TCP/TLS/Memory传输）
├── api/           # HTTP API路由（Axum）
├── auth/          # JWT认证+mTLS
├── client/        # 客户端实现（HTTP/Oneshot/Message）
├── services/      # 业务服务（Cert/Auth/Activation）
├── db/            # SurrealDB操作
└── utils/         # 工具函数（错误处理/日志）
```

## WHERE TO LOOK
| 任务 | 位置 | 备注 |
|------|------|------|
| 服务器启动 | src/main.rs | 初始化流程 |
| 消息传输 | src/message/ | 3种Transport实现 |
| mTLS安全 | src/message/tcp_server.rs | 三层证书验证 |
| HTTP服务 | src/core/server.rs | Axum+HTTPS |
| 状态管理 | src/core/state.rs | Arc共享状态 |
| 消息处理 | src/message/processor.rs | 6种消息类型 |

## CONVENTIONS
- **轻量级设计**: 每客户端~400字节内存，无需复杂池化
- **离线优先**: SurrealDB嵌入式，断网可运行99%功能
- **简单可靠**: 3-4客户端场景，复杂优化无意义
- **mTLS必需**: 不允许明文TCP启动

## ANTI-PATTERNS
- ❌ 高并发优化：只处理3-4客户端，无需性能调优
- ❌ 复杂缓存：数据量小，直接DB查询即可
- ❌ 负载均衡：单节点设计，无需分布式
- ❌ 微服务拆分：边缘节点应自包含

## 关键文件
- `src/message/bus.rs`: 消息总线核心，广播+单播
- `src/core/state.rs`: ServerState持有所有服务Arc引用
- `src/message/transport/`: TCP/TLS/Memory三种传输层
- `src/api/`: HTTP API端点，/health, /api/*

## 性能特性（3-4客户端）
- **内存**: 每连接400字节，总计<2MB
- **延迟**: 5-15ms简单命令响应
- **并发**: 同步处理足够，无需tokio复杂模式
- **消息队列**: 1024条容量，broadcast channel

## 注意事项
- **ServerState初始化**: 必须调用`initialize().await` + `start_background_tasks()`
- **TLS配置**: aws-lc-rs FIPS合规，仅TLS 1.3
- **错误处理**: PoC阶段unwrap()允许，生产需迁移到Result<T,E>
- **硬件绑定**: 证书device_id校验，防止克隆