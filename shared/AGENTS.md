# SHARED MODULE

**Generated:** 2026-01-16T18:07:10Z
**Reason:** 协议定义+共享类型，1500行代码，6种消息类型

## OVERVIEW
共享协议定义和类型，边缘服务器与客户端通信协议

## STRUCTURE
```
shared/src/
├── message/        # 消息协议定义（6种事件类型）
├── lib.rs         # 模块导出+共享类型
└── error.rs        # 统一错误类型
```

## WHERE TO LOOK
| 任务 | 位置 | 备注 |
|------|------|------|
| 消息协议 | src/message/mod.rs | 6种事件类型定义 |
| 序列化 | src/message/mod.rs | 二进制协议格式 |
| 错误类型 | src/error.rs | 统一API错误码 |
| 共享类型 | src/lib.rs | 通用数据结构 |

## 消息类型
| 类型 | 方向 | 用途 |
|------|------|------|
| Handshake | C→S | 握手验证（协议版本+身份） |
| RequestCommand | C→S | 客户端RPC请求（ping/echo/status） |
| Response | S→C | 请求响应（带correlation_id） |
| Notification | S→C | 系统通知/日志 |
| ServerCommand | Upstream→S | 上层服务器指令 |
| Sync | S→C | 数据同步信号 |

## 协议格式
```
[u8: event_type] + [u64: request_id] + [u64: correlation_id] + [Vec<u8>: payload]
```

## CONVENTIONS
- **强类型**: 所有消息在shared中定义，编译时类型安全
- **二进制协议**: 自定义wire格式，高效传输
- **序列化**: serde_json用于payload，bincode用于内部数据
- **错误码**: E1xxx认证，E2xxx权限，E3xxx令牌

## ANTI-PATTERNS
- ❌ 过度抽象：简单协议无需复杂设计
- ❌ 版本兼容：内部系统，版本锁定即可
- ❌ 性能优化：3-4客户端，协议简单优先

## 关键类型
- `BusMessage`: 统一消息封装
- `EventType`: 6种事件类型枚举
- `RequestId/CorrelationId`: 消息追踪
- `AppError`: 统一错误类型

## 注意事项
- **payload处理**: 使用serde序列化，大小限制
- **错误传播**: Result<T, E>模式，匹配HTTP状态码
- **类型安全**: 避免`serde_json::Value`，使用具体类型
- **向后兼容**: 新字段需添加可选字段