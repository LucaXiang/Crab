# Canonical Hash Chain Design

**Date**: 2026-02-27
**Status**: Approved

## Problem

Hash chain 计算依赖 `serde_json::to_string(&event.payload)` 的输出。这意味着：

1. **字段顺序敏感**：struct 字段重新排列后 JSON 输出不同 → hash 变化
2. **skip_serializing_if 行为**：Option/Vec 空值的序列化/跳过行为影响 JSON
3. **f64 序列化**：虽然 `ryu` 保证 roundtrip，但依赖外部 crate 行为
4. **无 roundtrip 测试**：没有验证 serialize → deserialize → serialize 一致性

## Solution: CanonicalHash Trait

完全脱离 serde，使用二进制协议生成确定性字节序列用于 hash 计算。

### Trait 定义

位置：`shared/src/order/canonical.rs`

```rust
pub trait CanonicalHash {
    fn canonical_bytes(&self, buf: &mut Vec<u8>);
}
```

### 编码规范

| 类型 | 编码 |
|------|------|
| `i64` / `u64` | 小端 8 字节 |
| `i32` / `u32` | 小端 4 字节 |
| `f64` | `to_bits()` → 小端 8 字节 |
| `String` | 长度 (u32 LE) + UTF-8 字节 |
| `bool` | 0x00 / 0x01 |
| `Option<T>` | 0x00 (None) 或 0x01 + T.canonical_bytes() |
| `Vec<T>` | 长度 (u32 LE) + 逐个元素 |
| `BTreeMap<K,V>` | 长度 (u32 LE) + 按 key 排序逐个 (K, V) |
| 枚举 variant | ASCII tag (如 `b"TABLE_OPENED"`) |
| 字段分隔 | `\x00` |

### 需要实现的类型

- `EventPayload` (26 variants) — 事件 hash
- `OrderEventType` — 事件 hash
- `OrderStatus` — 订单 hash
- `CartItemSnapshot`, `ItemOption`, `SpecificationInfo` — EventPayload 内嵌
- `PaymentRecord`, `SplitItem`, `ItemChanges`, `ItemModificationResult` — EventPayload 内嵌
- `PaymentSummaryItem` — OrderCompleted
- `AppliedRule`, `AppliedMgRule`, `MgItemDiscount` — 规则追踪
- `VoidType`, `LossReason`, `ServiceType` — 枚举
- `CompRecord`, `StampRedemptionState` — OrderSnapshot 内嵌

### Hash 计算改造

**事件 hash** (`compute_event_hash`):
```
SHA256(event_id || \x00 || order_id || \x00 || sequence_le || event_type_canonical || \x00 || payload_canonical)
```

**订单 hash** (`compute_order_hash`):
```
SHA256(prev_hash || \x00 || order_id || \x00 || receipt_number || \x00 || status_canonical || \x00 || last_event_hash)
```

### 测试策略

1. **Roundtrip 测试**：构造完整 EventPayload → serde roundtrip → canonical_bytes 不变
2. **Golden test**：固定输入 → 固定 hash 值，防回归
3. **Exhaustive match**：编译器强制新 variant 实现

### 迁移

现阶段开发阶段，旧数据可丢弃，不需要 hash_version 迁移机制。
