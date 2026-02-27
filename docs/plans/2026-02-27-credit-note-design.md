# Credit Note（退款凭证）设计文档

## 目标

为已归档订单提供追加式退款能力，不修改任何原始记录。所有修正通过追加 Credit Note 实现，与订单共享同一条 hash chain，保证不可篡改。

## 核心原则

1. **不可变性**：archived_order 和 archived_order_event 永远只读
2. **追加式修正**：退款通过新增 credit_note 记录实现
3. **共享 hash chain**：Order 和 Credit Note 进入同一条链（chain_entry 表）
4. **防超退**：系统追踪累计退款额，不允许超过原始订单总额

## 架构变化

### 之前

```
archived_order (prev_hash, curr_hash) → 云端同步
  └─ archived_order_event (prev_hash, curr_hash)
system_state.last_order_hash → 链头
```

### 之后

```
chain_entry (唯一 hash 链)
  ├─ entry_type=ORDER       → archived_order (纯业务数据，无 hash)
  │                            └─ archived_order_event (事件级 hash 链保留)
  └─ entry_type=CREDIT_NOTE → credit_note (纯业务数据，无 hash)
                                └─ credit_note_item (退款明细)
system_state.last_chain_hash → 链头（重命名 last_order_hash）
```

## 数据模型

### chain_entry（hash 链索引 — 唯一的链真相）

```sql
CREATE TABLE chain_entry (
    id          INTEGER PRIMARY KEY,     -- 自增，链的绝对顺序
    entry_type  TEXT    NOT NULL,         -- 'ORDER' | 'CREDIT_NOTE'
    entry_pk    INTEGER NOT NULL,         -- 指向对应业务表的 id
    prev_hash   TEXT    NOT NULL,         -- 上一条 chain_entry 的 curr_hash
    curr_hash   TEXT    NOT NULL,         -- 本条的 hash
    created_at  INTEGER NOT NULL          -- Unix millis
);

CREATE INDEX idx_chain_entry_created ON chain_entry(created_at);
CREATE INDEX idx_chain_entry_type ON chain_entry(entry_type);
```

### credit_note（退款凭证）

```sql
CREATE TABLE credit_note (
    id                    INTEGER PRIMARY KEY,
    credit_note_number    TEXT    NOT NULL,          -- CN-YYYYMMDD-NNNN
    original_order_pk     INTEGER NOT NULL REFERENCES archived_order(id),
    original_receipt      TEXT    NOT NULL,          -- 冗余存，查询方便

    -- 金额（正数，表示退了多少）
    subtotal_credit       REAL    NOT NULL,
    tax_credit            REAL    NOT NULL,
    total_credit          REAL    NOT NULL,          -- subtotal_credit + tax_credit

    -- 退款方式
    refund_method         TEXT    NOT NULL,          -- CASH | CARD

    -- 审计
    reason                TEXT    NOT NULL,
    note                  TEXT,
    operator_id           INTEGER NOT NULL,
    operator_name         TEXT    NOT NULL,
    authorizer_id         INTEGER,
    authorizer_name       TEXT,

    -- 归属
    shift_id              INTEGER REFERENCES shift(id),
    cloud_synced          INTEGER NOT NULL DEFAULT 0,
    created_at            INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_cn_number ON credit_note(credit_note_number);
CREATE INDEX idx_cn_original ON credit_note(original_order_pk);
CREATE INDEX idx_cn_created ON credit_note(created_at);
CREATE INDEX idx_cn_cloud_synced ON credit_note(cloud_synced);
CREATE INDEX idx_cn_shift ON credit_note(shift_id);
```

### credit_note_item（退款明细）

```sql
CREATE TABLE credit_note_item (
    id                    INTEGER PRIMARY KEY,
    credit_note_id        INTEGER NOT NULL REFERENCES credit_note(id),
    original_instance_id  TEXT    NOT NULL,          -- 原 CartItemSnapshot.instance_id
    item_name             TEXT    NOT NULL,
    quantity              INTEGER NOT NULL,           -- 退了几个
    unit_price            REAL    NOT NULL,           -- 原单价
    line_credit           REAL    NOT NULL,           -- quantity * unit_price
    tax_rate              INTEGER NOT NULL,           -- 原税率
    tax_credit            REAL    NOT NULL            -- 该项退税额
);

CREATE INDEX idx_cni_credit_note ON credit_note_item(credit_note_id);
```

### archived_order 变化

- **删除** `prev_hash`, `curr_hash` 列
- hash 信息迁移到 `chain_entry`
- `archived_order_event` 的事件级 hash 链保留（验证事件内部完整性）

### system_state 变化

- `last_order_hash` 重命名为 `last_chain_hash`（语义更准确）

## Hash Chain 计算

### Order 进链

```rust
// 归档订单时，在同一个事务内：
let prev_hash = system_state.last_chain_hash;
let curr_hash = compute_order_chain_hash(prev_hash, order_id, receipt_number, status, last_event_hash);
INSERT chain_entry (entry_type='ORDER', entry_pk=order.id, prev_hash, curr_hash);
UPDATE system_state SET last_chain_hash = curr_hash;
```

### Credit Note 进链

```rust
// 创建退款凭证时，在同一个事务内：
let prev_hash = system_state.last_chain_hash;
let curr_hash = compute_credit_note_chain_hash(prev_hash, cn_number, original_receipt, total_credit);
INSERT chain_entry (entry_type='CREDIT_NOTE', entry_pk=cn.id, prev_hash, curr_hash);
UPDATE system_state SET last_chain_hash = curr_hash;
```

### compute_credit_note_chain_hash

```rust
// shared/src/order/canonical.rs
pub fn compute_credit_note_chain_hash(
    prev_hash: &str,
    credit_note_number: &str,
    original_receipt: &str,
    total_credit: f64,
) -> String {
    let mut buf = Vec::with_capacity(256);
    write_str(&mut buf, prev_hash);
    write_str(&mut buf, credit_note_number);
    write_str(&mut buf, original_receipt);
    write_f64(&mut buf, total_credit);
    format!("{:x}", Sha256::digest(&buf))
}
```

## 验证逻辑

### 日常验证（VerifyScheduler）

```sql
-- 只查 chain_entry，不需要 UNION 两张表
SELECT id, entry_type, entry_pk, prev_hash, curr_hash
FROM chain_entry
WHERE created_at BETWEEN ? AND ?
ORDER BY id ASC;
```

逐行验证：`row[i].prev_hash == row[i-1].curr_hash`

### 单订单事件链验证

不变 — 仍然查 `archived_order_event` 的 prev_hash/curr_hash 链。

## 防超退

```rust
async fn get_refundable_info(order_pk: i64) -> RefundableInfo {
    let order = get_archived_order(order_pk);
    let total_refunded: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_credit), 0.0) FROM credit_note WHERE original_order_pk = ?"
    ).bind(order_pk).fetch_one(&pool).await;

    RefundableInfo {
        original_total: order.total_amount,
        already_refunded: total_refunded,
        remaining_refundable: order.total_amount - total_refunded,
    }
}

// 创建退款前校验
if request.total_credit > refundable_info.remaining_refundable {
    return Err(AppError::validation("退款金额超过可退余额"));
}
```

## API 设计

### POST /api/credit-notes

```rust
pub struct CreateCreditNoteRequest {
    pub original_order_pk: i64,
    pub items: Vec<CreditNoteItemRequest>,  // 退哪些商品
    pub refund_method: String,              // CASH | CARD
    pub reason: String,
    pub note: Option<String>,
    pub authorizer_id: Option<i64>,         // Manager 授权
}

pub struct CreditNoteItemRequest {
    pub instance_id: String,                // 原商品 instance_id
    pub quantity: i32,                      // 退几个
}
```

### GET /api/orders/:id/credit-notes

返回该订单的所有退款记录。

### GET /api/credit-notes/:id

返回单条退款凭证详情（含 items）。

## 退款小票

```
================================
        NOTA DE CRÉDITO
================================
No: CN-20260227-0001
Fecha: 27/02/2026 14:35
Factura original: FAC20260227-0001
--------------------------------
Artículo         Cant.   Importe
--------------------------------
Paella             x1      12.50
Cerveza            x1       3.50
--------------------------------
Subtotal:                  13.22
IVA (21%):                  2.78
TOTAL DEVUELTO:            16.00
--------------------------------
Método: EFECTIVO
Motivo: Calidad del producto
Autorizado: Manager
Cajero: María
================================
```

新增 `CreditNoteRenderer`（与 `KitchenTicketRenderer` 平级）。

## 云端同步

与 archived_order 同步方式完全一致：

1. credit_note 创建时 `cloud_synced = 0`
2. CloudWorker 按 `chain_entry.id` 顺序同步：
   - ORDER → 发送 archived_order 数据
   - CREDIT_NOTE → 发送 credit_note + credit_note_item 数据
3. 云端确认后 `cloud_synced = 1`
4. 云端独立验证 chain_entry 的 hash 连续性

## 班次影响

- Credit Note 归属到**发起退款时的当前班次**（不是原始订单的班次）
- 退现金时：`shift.expected_cash -= total_credit`
- 班次报表中显示退款汇总

## 迁移计划

### 数据迁移（archived_order → chain_entry）

```sql
-- 1. 创建 chain_entry 表
-- 2. 将现有 archived_order 的 hash 数据迁移进去
INSERT INTO chain_entry (entry_type, entry_pk, prev_hash, curr_hash, created_at)
SELECT 'ORDER', id, prev_hash, curr_hash, created_at
FROM archived_order
ORDER BY id ASC;

-- 3. 重命名 system_state 字段
-- last_order_hash → last_chain_hash

-- 4. 删除 archived_order 的 prev_hash, curr_hash 列
-- （SQLite 不支持 DROP COLUMN on older versions，可能需要重建表）
```

## 实现范围

### Phase 1: 基础设施（chain_entry + 迁移）
- 新增 chain_entry 表
- 迁移现有 hash 数据
- 重构 OrderArchiveService 使用 chain_entry
- 重构 VerifyScheduler 使用 chain_entry
- 重构 CloudWorker 同步 chain_entry

### Phase 2: Credit Note 核心
- 新增 credit_note / credit_note_item 表
- CreditNoteService（创建、查询、防超退）
- compute_credit_note_chain_hash
- API 端点

### Phase 3: 打印 + 前端
- CreditNoteRenderer（ESC/POS 退款小票）
- 前端：历史订单详情页显示退款记录
- 前端：退款操作流程 UI
- 班次现金追踪

### Phase 4: 云端
- cloud 端 credit_note 存储表
- CloudWorker 同步 credit_note
- cloud 端 chain_entry 验证
- Console 管理后台显示退款记录

## Verifactu 兼容性设计

### 映射关系

| chain_entry.entry_type | 业务含义 | Verifactu 操作 | SII 类型 |
|------------------------|----------|----------------|----------|
| `ORDER` | 销售订单归档 | **Alta** (高开/创建) | Factura emitida |
| `CREDIT_NOTE` | 退款/修正凭证 | **Rectificativa** (修正发票) | Factura rectificativa |
| `ORDER_VOID` (预留) | 整单作废 | **Anulación** (取消) | Anulación de factura |

### entry_type 可扩展性

`chain_entry.entry_type` 是 `TEXT`，当前值：
- `ORDER` — 正常销售
- `CREDIT_NOTE` — 退款修正

未来 Verifactu 对接时新增：
- `ORDER_VOID` — 整单取消（Anulación），对应已存在的 `OrderStatus::Void`

### 发票号映射

```
receipt_number (FAC20260227-0001)  →  Verifactu NúmeroFactura
credit_note_number (CN-20260227-0001) →  Verifactu NúmeroFacturaRectificativa
```

### chain_entry → Verifactu 链

Verifactu 要求每张发票引用前一张发票的 hash（`EncadenamientoFacturaAnterior`）。`chain_entry` 的 `prev_hash / curr_hash` 天然满足这个需求：

```
chain_entry[0] ORDER       prev=GENESIS  curr=H1  → Alta #1
chain_entry[1] ORDER       prev=H1       curr=H2  → Alta #2
chain_entry[2] CREDIT_NOTE prev=H2       curr=H3  → Rectificativa (引用 Alta #2)
chain_entry[3] ORDER_VOID  prev=H3       curr=H4  → Anulación (引用 Alta #1)
```

### credit_note 表预留字段（Phase 2 暂不添加）

Verifactu 对接时需要的字段，记录在此作为设计参考，不在 Phase 2 实现：

```sql
-- 以下字段在 Verifactu 对接阶段添加
verifactu_id          TEXT,     -- Verifactu 注册 ID
verifactu_status      TEXT,     -- PENDING | SUBMITTED | ACCEPTED | REJECTED
verifactu_submitted_at INTEGER, -- 提交时间
rectificativa_type    TEXT,     -- 'I' (差额) | 'S' (替代) — Verifactu 修正类型
```

### 设计要点

1. **chain_entry 是转换入口**：生成 Verifactu XML 时，遍历 chain_entry 即可按顺序输出所有发票和修正
2. **credit_note.original_receipt** 直接映射为修正发票的原始发票引用
3. **hash chain 天然满足 Verifactu 链式要求**，无需额外数据结构
4. **entry_type TEXT 而非 ENUM**：便于未来扩展新类型（如 ORDER_VOID）而无需迁移
