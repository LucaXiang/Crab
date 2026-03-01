# 全链路类型对齐 + 同步补全 + PG Schema 修正

**目标**: 消除 Edge→Shared→Cloud→Frontend 全链路中的所有类型断裂、数据丢失和 schema 技术债。

**原则**: 开发阶段，不要适配层，不要兼容性，从 source 向外修。

---

## 一、类型层对齐 (T1-T5)

### T1: creditNote.ts cloud_synced 类型错误
- `red_coral/src/core/domain/types/creditNote.ts:24`: `cloud_synced: number` → `boolean`

### T2: invoice.ts 枚举成员缺失
- `red_coral/src/core/domain/types/invoice.ts:8`: `TipoFactura` 加 `'F3'`
- `red_coral/src/core/domain/types/invoice.ts:11`: `InvoiceSourceType` 加 `'UPGRADE'`

### T3: Sync 载荷 String → typed enum
- `shared/src/cloud/sync.rs` `OrderDetailPayload`:
  - `void_type: Option<String>` → `Option<VoidType>`
  - `loss_reason: Option<String>` → `Option<LossReason>`
  - `service_type: Option<String>` → `Option<ServiceType>`
- `VoidType`、`LossReason`、`ServiceType` 已在 `shared::order::types` 中定义
- Edge `build_order_detail_sync`: SQLite TEXT → serde 反序列化为 enum
- Cloud `sync_store.rs`: enum `.as_str()` 存 PG TEXT（或 `serde_json::to_string`）
- **删除** `sync_store.rs:508` 的 `"DineIn"` 硬编码 fallback → 存 NULL

### T4: AnulacionSync.reason String → AnulacionReason
- `shared/src/cloud/sync.rs:460`: `reason: String` → `reason: AnulacionReason`
- Edge `build_sync`: 直接用 enum 构建
- Cloud `sync_store.rs`: `.as_str()` 存 PG

### T5: 统一 desglose 类型
- 废弃 `shared/src/models/invoice.rs` 中的 `InvoiceDesglose` struct
- 全局统一使用 `shared/src/cloud/sync.rs` 中的 `TaxDesglose` (`i32` + `Decimal`)
- Edge 侧 `invoice.rs` / `credit_note.rs` 读 SQLite f64 后 `Decimal::try_from()`
- `InvoiceDesglose.tax_rate: i64` → 统一为 `TaxDesglose.tax_rate: i32`

---

## 二、同步管道补全 (S1-S5)

### S1: 订单 created_at 丢失
- Cloud PG migration: `store_archived_orders` ADD `created_at BIGINT`
- `sync_store.rs`: INSERT 绑定 `OrderDetailSync.created_at`

### S2: fecha_hora_registro 丢失
- Cloud PG migration:
  - `store_invoices` ADD `fecha_hora_registro TEXT`
  - `store_anulaciones` ADD `fecha_hora_registro TEXT`
- `sync_store.rs`: INSERT 绑定对应字段

### S3: 订单补全 queue_number / shift_id / operator_id / member_id
- `shared/src/cloud/sync.rs` `OrderDetailPayload` 添加:
  - `queue_number: Option<String>`
  - `shift_id: Option<i64>`
  - `operator_id: Option<i64>`
  - `member_id: Option<i64>`
- Edge `build_order_detail_sync`: 从 SQLite SELECT 读取
- Cloud PG migration: `store_archived_orders` ADD 4 列
- `sync_store.rs`: INSERT 绑定

### S4: CreditNoteItemSync 补全 original_instance_id
- `shared/src/cloud/sync.rs` `CreditNoteItemSync` 添加 `original_instance_id: String`
- Edge `build_sync`: 从 SQLite 读取
- Cloud PG migration: `store_credit_note_items` ADD `original_instance_id TEXT`
- `sync_store.rs`: INSERT 绑定

### S5: OrderPaymentSync 补全支付详情
- `shared/src/cloud/sync.rs` `OrderPaymentSync` 添加:
  - `cancel_reason: Option<String>`
  - `tendered: Option<f64>`
  - `change_amount: Option<f64>`
- Edge `build_order_detail_sync`: 从 SQLite SELECT 读取
- Cloud PG migration: `store_order_payments` ADD 3 列
- `sync_store.rs`: INSERT 绑定

---

## 三、PG Schema 修正 (P1-P4)

### P1: 所有货币列 DOUBLE PRECISION → NUMERIC(12,2)
涉及表:
- `store_archived_orders`: total, tax, discount_amount, loss_amount, original_total, subtotal, paid_amount, surcharge_amount, comp_total_amount, order_manual_discount_amount, order_manual_surcharge_amount, order_rule_discount_amount, order_rule_surcharge_amount
- `store_order_items`: price, unit_price, line_total, discount_amount, surcharge_amount, tax
- `store_order_payments`: amount (+ 新加 tendered, change_amount)
- `store_credit_notes`: subtotal_credit, tax_credit, total_credit
- `store_credit_note_items`: unit_price, line_credit, tax_credit
- `store_invoices`: subtotal, tax, total

`sync_store.rs` 所有 `.bind(f64)` → `.bind(Decimal::try_from(f64).unwrap_or_default())`
`tenant_queries.rs` 读回 `Decimal` → `.to_f64()` 返回 API

### P2: source_id TEXT → BIGINT
- `store_archived_orders.source_id`: `TEXT NOT NULL` → `BIGINT NOT NULL`

### P3: InvoiceSync serde 一致性
- `InvoiceSync` optional 字段加 `#[serde(skip_serializing_if = "Option::is_none")]`

### P4: ON CONFLICT SET 补全
- `upsert_credit_note`: ON CONFLICT 补全 subtotal_credit, tax_credit, refund_method, reason, note, operator_name, authorizer_name, original_receipt
- `upsert_invoice`: ON CONFLICT 补全 subtotal, tax, nif, nombre_razon, customer_*, fecha_expedicion, tipo_factura, source_type, fecha_hora_registro

---

## 四、执行顺序

1. Cloud PG migration (0008): 所有 ALTER + ADD COLUMN
2. Shared types: T3/T4/T5 类型改动
3. Edge: build_order_detail_sync + build_sync 补全字段
4. Cloud: sync_store.rs 适配新类型 + 补全 INSERT/UPDATE
5. Cloud: tenant_queries.rs 适配 Decimal
6. Frontend: T1/T2 TS 类型修正
7. 验证: `cargo clippy --workspace` + `cd red_coral && npx tsc --noEmit`

## 五、验证清单

- [ ] `cargo check --workspace` 零错误
- [ ] `cargo clippy --workspace` 零警告
- [ ] `cargo test -p edge-server --lib` 全部通过
- [ ] `cd red_coral && npx tsc --noEmit` 零错误
- [ ] 全链路每个字段从 Edge SQLite → Shared type → Cloud PG → tenant_queries 都有对应
