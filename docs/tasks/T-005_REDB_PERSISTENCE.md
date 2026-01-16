# T-005/006: MessageBus + redb 持久化 - 任务追踪

> **任务 ID**: T-005, T-006
> **优先级**: P1
> **预估工时**: 20h
> **实际工时**: 6h
> **状态**: ✅ 已完成 (完整版)

## 任务描述

**目标**: 引入 redb 实现 MessageBus 持久化，确保消息可靠性

**已实现功能** (RequestStore):
1. ✅ 全消息持久化 (WAL - Write-Ahead Log)
2. ✅ 幂等性检查 (基于 request_id 防重复)
3. ✅ 消费者偏移量管理 (支持消息重放)
4. ✅ 死信队列 (DLQ) - 失败消息存储
5. ✅ 消息重放机制 (从指定偏移重新消费)
6. ✅ 基于 redb 3.1 + bincode2 持久化
7. ✅ 集成到 MessageBus 的 `send_to_server` 方法

## 架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                    MessageBus + RequestStore                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌─────────────────────────────────┐    │
│  │  Transport Layer│    │   RequestStore (redb 3.1)       │    │
│  │  (TCP/TLS/Mem)  │───▶│  - WAL (Write-Ahead Log)        │    │
│  │                 │    │  - 幂等性检查 (Idempotency)     │    │
│  │                 │    │  - 消费者偏移量 (Offsets)       │    │
│  │                 │    │  - 死信队列 (DLQ)               │    │
│  │                 │    │  - 消息重放 (Replay)            │    │
│  └─────────────────┘    └─────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 已实现

| 文件 | 功能 |
|------|------|
| `edge-server/src/message/store.rs` | RequestStore 持久化层 (WAL + DLQ + Offset + Replay) |
| `edge-server/src/message/mod.rs` | MessageBus 集成 |
| `edge-server/src/server/services/message_bus.rs` | MessageBusService 初始化 |

## redb 表结构

| 表名 | Key | Value | 说明 |
|------|-----|-------|------|
| `request_log` | u64 (seq) | PersistedRequest | WAL 主存储 |
| `request_idempotency` | [u8; 16] (UUID) | u64 (seq) | 幂等性检查 |
| `consumer_offsets` | &str (consumer_id) | u64 (seq) | 消费者偏移量 |
| `dead_letters` | u64 (seq) | PersistedRequest | 死信队列 |
| `metadata` | &str | u64 | 元数据 (last_seq) |

## 变更清单

- [x] 添加 redb 依赖 (redb = "3.1")
- [x] 添加 bincode2 依赖 (bincode2 = "2")
- [x] 创建 RequestStore 结构
- [x] 实现 WAL (append_request, get_by_seq)
- [x] 实现幂等性检查 (is_duplicate)
- [x] 实现消费者偏移量 (get/set_consumer_offset)
- [x] 实现死信队列 (move_to_dead_letter, dead_letter_count)
- [x] 实现消息重放 (replay_from)
- [x] 集成到 MessageBus
- [x] 编写完整测试 (4 tests, all passed)

## 依赖

```toml
# Cargo.toml
[dependencies]
redb = "3.1"
bincode2 = "2"
```

## 测试结果

```
running 4 tests
test message::store::tests::test_request_store_wal ... ok
test message::store::tests::test_consumer_offsets ... ok
test message::store::tests::test_dead_letter_queue ... ok
test message::store::tests::test_replay ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out
```

---

## 更新日志

| 日期 | 操作 | 更新人 |
|------|------|--------|
| 2026-01-16 | 创建任务追踪 | - |
| 2026-01-16 | 完成 MessageCache 基础实现 | Claude |
| 2026-01-16 | 完成 RequestStore 完整实现 (WAL + DLQ + Offset + Replay) | Claude |
