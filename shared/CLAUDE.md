# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Shared

跨 crate 共享类型、协议定义、错误系统。

## 命令

```bash
cargo check -p shared
cargo test -p shared --lib
```

## 模块结构

```
src/
├── models/     # 数据模型 (与前端 TypeScript 对齐)
├── order/      # 订单类型和事件定义
├── message/    # 消息总线协议 (BusMessage, EventType)
├── error/      # 统一错误系统 (ErrorCode, AppError)
├── intent/     # DataIntent 分发模式
├── activation/ # 设备激活协议
├── request.rs  # 请求类型
├── response.rs # 响应类型
└── types.rs    # 通用类型
```

## 核心类型

### BusMessage / EventType

消息总线协议，支持:
- `Handshake`: 握手
- `RequestCommand`: RPC 请求
- `Response`: 响应
- `Notification`: 通知
- `Sync`: 数据同步

### ErrorCode

统一错误码 (u16)，前端/后端共享:
```rust
pub enum ErrorCode {
    Success = 0,
    ValidationError = 1001,
    NotFound = 1002,
    // ...
}
```

## 类型对齐

修改 `models/` 时，必须同步更新:
- 前端: `red_coral/src/core/domain/types/api/models.ts`

## 响应语言

使用中文回答。
