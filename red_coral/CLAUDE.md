# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## RedCoral POS

Tauri 2 + React 19 + TypeScript + Rust 全栈 POS 应用。

## 命令

```bash
npm run tauri:dev      # 开发
npm run tauri:build    # 构建
npx tsc --noEmit       # TS 类型检查
npm run test           # Vitest 测试
```

## 项目结构

```
red_coral/
├── src/                          # React 前端
│   ├── core/
│   │   ├── stores/               # Zustand 状态管理
│   │   │   ├── order/            # 订单 (Event Sourcing)
│   │   │   ├── cart/             # 购物车
│   │   │   ├── resources/        # 资源 Store (Product, Category, etc.)
│   │   │   └── auth/             # 认证状态
│   │   ├── domain/types/         # TypeScript 类型定义
│   │   └── hooks/                # 核心 Hooks
│   ├── screens/                  # 页面组件
│   ├── presentation/components/  # UI 组件
│   ├── infrastructure/api/       # Tauri API 客户端
│   └── utils/currency/           # 金额计算 (Decimal.js)
└── src-tauri/                    # Rust 后端
    ├── src/
    │   ├── core/bridge/          # ClientBridge (Server/Client 双模式)
    │   ├── commands/             # Tauri Commands
    │   └── utils/                # 打印、收据渲染
    └── Cargo.toml
```

## 核心概念

### ClientBridge 双模式

`src-tauri/src/core/bridge/mod.rs`:
- **Server 模式**: 内嵌 edge-server，In-Process 通信
- **Client 模式**: mTLS 连接远程 edge-server

### 订单系统 (Event Sourcing)

```
OrderCommand → edge-server → OrderEvent → 广播 → 前端 Store 更新
```

- `useOrderCommands`: 发送命令
- `useActiveOrdersStore`: 订单快照状态
- `orderReducer`: Event → Snapshot 转换

### 类型对齐

TypeScript 类型必须与 Rust 完全匹配：
- Rust: `edge-server/src/db/models/`, `shared/src/`
- TypeScript: `src/core/domain/types/api/models.ts`

修改流程: Rust → TypeScript → `npx tsc --noEmit` 验证

### 金额计算

必须使用 `Currency` 工具类：
```typescript
import { Currency } from '@/utils/currency';
Currency.add(a, b);
Currency.floor2(total);
```

## 添加 Tauri Command

1. `src-tauri/src/commands/` 添加函数
2. `src-tauri/src/lib.rs` 注册到 invoke_handler
3. 前端调用: `invoke<T>('command_name', { args })`

## 数据目录

`~/Library/Application Support/com.xzy.pos/redcoral/`
- `config.json` - 模式和租户配置
- `tenants/` - 租户证书存储
- `database/` - 本地数据库

## 响应语言

使用中文回答。
