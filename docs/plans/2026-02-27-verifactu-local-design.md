# Verifactu 本地基础设施设计

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在 edge-server 本地实现 Verifactu 发票生成、编号、huella 计算的完整基础设施，使云端同步后拥有向 AEAT 提交所需的一切数据。

**Architecture:** 发票在 archive 阶段自动生成（归档 hook），存入 SQLite，huella 独立于内部 chain hash。云端只负责 XML 组装 + AEAT 提交 + 按 chain 顺序严格提交。

**Tech Stack:** Rust (shared + edge-server), SQLite, SHA-256, rust_decimal

---

## 已确认的设计决策

| # | 决策 | 来源 |
|---|------|------|
| 1 | 默认所有小票 = F2 (Simplificada) | 用户确认 |
| 2 | Credit Note = R5 (Rectificativa en Simplificadas) | Verifactu 规范 |
| 3 | VOID 订单 = Anulación | Verifactu 规范 |
| 4 | 客户要完整发票 = F3 (Sustitución) | Verifactu 规范 |
| 5 | Comped (total=0) 不开发票 | 用户确认 |
| 6 | LossSettled 按比例缩放 desglose | 用户确认 |
| 7 | 拆单 = 分开付款，只开一个发票 | 用户确认 |
| 8 | 合单 = 合入菜品当加菜，目标订单开发票 | 用户确认 |
| 9 | NIF 从 P12 证书读取（权威来源） | 用户确认 |
| 10 | 历史订单不管，从实现日开始 | 用户确认 |
| 11 | fecha_expedicion = 自然日（非 business_date） | 用户确认 |
| 12 | F2 和 R5 共用同一 Serie + 序号 | 用户确认 |
| 13 | 云端严格按 chain 顺序提交 AEAT | 用户确认 |

## 类型映射

```
内部概念          → Verifactu TipoFactura → 说明
─────────────────────────────────────────────────
Completed 订单   → F2 (Simplificada)      → 正常销售
Credit Note      → R5 (Rectificativa)     → 部分/全额退款修正
VOID 订单        → Anulación              → 完全作废
升级完整发票      → F3 (Sustitución)       → 替代 F2（Phase 2）
```

## 数据模型

### SQLite 新表: `invoice`

```sql
CREATE TABLE invoice (
    id            INTEGER PRIMARY KEY,        -- snowflake
    invoice_number TEXT NOT NULL UNIQUE,       -- '{Serie}-{YYYYMMDD}-{NNNN}'
    serie         TEXT NOT NULL,              -- 'A', 'B', ... (基于 store_number)
    tipo_factura  TEXT NOT NULL,              -- 'F2', 'R5'
    -- 关联
    source_type   TEXT NOT NULL,              -- 'ORDER' | 'CREDIT_NOTE'
    source_pk     INTEGER NOT NULL,           -- archived_order.id 或 credit_note.id
    -- 发票金额 (从 source 复制，不可变)
    subtotal      REAL NOT NULL,              -- 不含税小计
    tax           REAL NOT NULL,              -- 税额
    total         REAL NOT NULL,              -- 含税总计
    -- Verifactu desglose (税目明细，JSON 不适合，用关联表)
    -- → invoice_desglose 表
    -- Verifactu huella (hash chain)
    huella        TEXT NOT NULL,              -- SHA-256 hex
    prev_huella   TEXT,                       -- 前一条 huella (NULL = 链首)
    -- 元数据
    fecha_expedicion TEXT NOT NULL,           -- 'YYYY-MM-DD' 自然日
    nif           TEXT NOT NULL,              -- 从 P12 读取
    nombre_razon  TEXT NOT NULL,              -- 商户名称
    -- R5 专用 (rectificativa)
    factura_rectificada_id  INTEGER,          -- 原 invoice.id (R5 引用的 F2)
    factura_rectificada_num TEXT,             -- 原 invoice_number
    -- 同步状态
    cloud_synced  INTEGER NOT NULL DEFAULT 0, -- 0=未同步, 1=已同步
    aeat_status   TEXT NOT NULL DEFAULT 'PENDING', -- PENDING | SUBMITTED | ACCEPTED | REJECTED
    -- 时间
    created_at    INTEGER NOT NULL            -- Unix millis
);

CREATE INDEX idx_invoice_source ON invoice(source_type, source_pk);
CREATE INDEX idx_invoice_cloud_synced ON invoice(cloud_synced);
CREATE INDEX idx_invoice_serie_number ON invoice(serie, invoice_number);
```

### SQLite 新表: `invoice_desglose`

```sql
CREATE TABLE invoice_desglose (
    id          INTEGER PRIMARY KEY,
    invoice_id  INTEGER NOT NULL REFERENCES invoice(id),
    tax_rate    INTEGER NOT NULL,    -- 基点 (1000 = 10.00%)
    base_amount REAL NOT NULL,       -- 税基 (不含税)
    tax_amount  REAL NOT NULL,       -- 税额
    UNIQUE(invoice_id, tax_rate)
);
```

### SQLite 新表: `invoice_counter`

```sql
CREATE TABLE invoice_counter (
    serie       TEXT PRIMARY KEY,
    date_str    TEXT NOT NULL,         -- 'YYYYMMDD' (当日)
    last_number INTEGER NOT NULL       -- 当日最后序号
);
```

双重校验逻辑：启动时取 `MAX(counter.last_number, MAX(实际 invoice 中同 serie 同日的序号))`，防止计数器故障导致重号。

## Huella (Verifactu Hash Chain)

### 格式

按 AEAT 官方规范（Registro de Alta），huella 使用 `key=value&` 拼接后 UTF-8 → SHA-256 → 64 字符 hex：

```
IDEmisorFactura={NIF}&NumSerieFactura={invoice_number}&FechaExpedicionFactura={DD-MM-YYYY}&TipoFactura={F2|R5}&CuotaTotal={税额}&ImporteTotal={含税总额}&Huella={prev_huella|空串}&FechaHoraHusoGenRegistro={ISO8601+TZ}
```

**规范要点**:
- 首条记录: `Huella=` (空值，不是省略)
- 金额: 去空格，小数规范化 (123.10 → 123.1)
- 时间: ISO 8601 带时区 `2026-02-27T19:20:30+01:00`
- 日期: `DD-MM-YYYY` (注意不是 YYYY-MM-DD)
- 结果: SHA-256 hex 小写 64 字符

**Anulación (Phase 2)** 只有 5 个字段:
```
IDEmisorFacturaAnulada={NIF}&NumSerieFacturaAnulada={原发票号}&FechaExpedicionFacturaAnulada={原日期}&Huella={prev}&FechaHoraHusoGenRegistro={ISO8601+TZ}
```

**注意**: 这是独立于内部 `chain_entry` 的第二条 hash chain，专用于 Verifactu 合规。

### shared 函数

```rust
// shared/src/order/verifactu.rs

/// Verifactu Registro de Alta huella 计算 (F2, R5)
pub fn compute_verifactu_huella_alta(
    nif: &str,                       // IDEmisorFactura
    invoice_number: &str,            // NumSerieFactura
    fecha_expedicion: &str,          // FechaExpedicionFactura (DD-MM-YYYY)
    tipo_factura: &str,              // TipoFactura (F2, R5)
    cuota_total: f64,                // CuotaTotal (税额)
    importe_total: f64,              // ImporteTotal (含税总额)
    prev_huella: Option<&str>,       // Huella (None → 空串)
    fecha_hora_registro: &str,       // FechaHoraHusoGenRegistro (ISO8601+TZ)
) -> String {
    // 1. 拼接 key=value& 字符串
    // 2. UTF-8 bytes → SHA-256
    // 3. 返回 hex 小写 64 字符
}

/// Verifactu Registro de Anulación huella 计算 (Phase 2)
pub fn compute_verifactu_huella_anulacion(
    nif: &str,                       // IDEmisorFacturaAnulada
    invoice_number: &str,            // NumSerieFacturaAnulada
    fecha_expedicion: &str,          // FechaExpedicionFacturaAnulada
    prev_huella: Option<&str>,       // Huella
    fecha_hora_registro: &str,       // FechaHoraHusoGenRegistro
) -> String;

/// 金额格式化: 去尾部零 (123.10 → "123.1", 100.00 → "100")
fn format_amount(v: f64) -> String;
```

## 发票编号

### 格式

```
{Serie}-{YYYYMMDD}-{NNNN}
```

- **Serie**: 基于 `store_number` → `A`=1, `B`=2, `C`=3, ...
- **YYYYMMDD**: 自然日（`chrono::Utc::now().with_timezone(&tz)`）
- **NNNN**: 当日该 Serie 的递增序号（从 0001 开始）
- **F2 和 R5 共用同一序号**：保证整个 Serie 内严格递增

示例：`A-20260227-0001` (F2), `A-20260227-0002` (R5), `A-20260227-0003` (F2)

### 与 receipt_number 的关系

| 编号 | 格式 | 用途 | 生成时机 |
|------|------|------|---------|
| `receipt_number` | `01-20260227-0001` | 内部小票号 | OpenTable 时 |
| `invoice_number` | `A-20260227-0001` | 税务发票号 | Archive 时 |
| `credit_note_number` | `CN-20260227-0001` | 退款凭证号 | Credit Note 创建时 |

三者独立递增，互不干扰。`invoice_number` 是对外的法定编号。

## 生成流程

### 正常订单归档 (Completed → F2)

```
OrderArchiveService::archive_order()
  ├─ 现有流程: INSERT archived_order + chain_entry
  ├─ 新增: InvoiceService::create_invoice_for_order()
  │   ├─ 检查: total > 0 (comped 不开票)
  │   ├─ 检查: source_type=ORDER, source_pk 无重复
  │   ├─ 分配 invoice_number (从 invoice_counter)
  │   ├─ 读取 NIF (从 P12 / ActivationData 缓存)
  │   ├─ 计算 desglose (从 archived_order_item)
  │   ├─ 计算 huella (读 system_state 中的 last_huella)
  │   ├─ INSERT invoice + invoice_desglose
  │   ├─ UPDATE system_state.last_huella
  │   └─ 全部在同一个 SQLite 事务中
  └─ commit
```

### VOID 订单 (Voided → Anulación)

```
OrderArchiveService::archive_order()
  ├─ 现有流程: INSERT archived_order + chain_entry
  ├─ 新增: 检查该订单是否已开过 F2
  │   ├─ 如果有 F2 → InvoiceService::create_anulacion()
  │   │   ├─ 不生成新 invoice 行
  │   │   ├─ 标记原 F2 的 aeat_status = 'ANULADA'
  │   │   └─ 生成 anulación huella (单独 chain entry)
  │   └─ 如果没有 F2 (VOID 在 archive 前) → 跳过
  └─ commit
```

**注意**: 当前系统中，VOID 和 Completed 都走 archive 流程。VOID 的 `status=Void`，但如果这个订单之前已经 Completed 并开过 F2，则需要 Anulación。但实际上我们当前的流程是：订单只会 archive 一次（Completed 或 Void），不会先 Complete 再 Void。所以:
- **Completed → archive + F2**
- **Void → archive + 跳过发票**（未完成的订单作废不涉及税务）

### Credit Note (退款 → R5)

```
CreditNoteService::create_credit_note()
  ├─ 现有流程: INSERT credit_note + chain_entry
  ├─ 新增: InvoiceService::create_invoice_for_credit_note()
  │   ├─ 查找原订单的 F2 invoice
  │   ├─ 分配 invoice_number (同一 Serie + 序号)
  │   ├─ tipo_factura = 'R5'
  │   ├─ factura_rectificada_id = 原 F2 的 invoice.id
  │   ├─ 金额为负 (或绝对值，按 Verifactu 规范)
  │   ├─ 计算 huella
  │   ├─ INSERT invoice + invoice_desglose
  │   └─ UPDATE system_state.last_huella
  └─ commit
```

### LossSettled 订单的 desglose

当订单 `void_type = LossSettled` 时，`total_amount < original_total`。desglose 按比例缩放：

```rust
// 比例因子 = total_amount / original_total
// 每个税率: base_amount *= factor, tax_amount *= factor
```

但 LossSettled 的订单状态是 Void，按上面的逻辑不会开发票。如果 LossSettled 需要开发票（部分结算），需要额外处理 — **Phase 2 再考虑**。

## NIF 来源

从 P12 证书中提取 NIF（CIF），缓存在内存中：

```rust
// edge-server 启动时或首次需要时
// P12 路径: {work_dir}/certs/signing.p12
// 解析 X.509 subject → serialNumber 字段 = NIF
```

**Phase 1 简化**: 从 `StoreInfo.nif` 读取（已有字段），确保 cloud 在激活时从 P12 提取并写入 `StoreInfo.nif`。真正的 P12 读取逻辑在 Phase 2。

## 云端同步

### InvoiceSync 类型 (shared)

```rust
pub struct InvoiceSync {
    pub id: i64,
    pub invoice_number: String,
    pub serie: String,
    pub tipo_factura: String,       // "F2", "R5"
    pub source_type: String,
    pub source_pk: i64,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub desglose: Vec<TaxDesglose>,
    pub huella: String,
    pub prev_huella: Option<String>,
    pub fecha_expedicion: String,
    pub nif: String,
    pub nombre_razon: String,
    pub factura_rectificada_id: Option<i64>,
    pub factura_rectificada_num: Option<String>,
    pub created_at: i64,
}
```

### 云端严格顺序提交

云端收到 InvoiceSync 后：
1. 存入 `store_invoice` 表
2. Verifactu 提交队列 per store，严格按 `invoice_number` 顺序
3. 失败则阻塞后续提交，直到重试成功或人工干预
4. 提交成功后回写 `aeat_status = 'ACCEPTED'`

**这部分在 crab-cloud 实现，不在本设计范围内。**

## system_state 扩展

```sql
ALTER TABLE system_state ADD COLUMN last_huella TEXT;
```

与 `last_chain_hash` 并列，两条独立的 hash chain。

## 锁机制

`InvoiceService` 共享 `hash_chain_lock`（与 OrderArchiveService、CreditNoteService 相同的 `Arc<Mutex<()>>`），因为发票生成发生在 archive/credit_note 事务内部，已经在锁内。

## 文件变更清单

| 文件 | 变更 |
|------|------|
| **新建** `shared/src/order/verifactu.rs` | `compute_verifactu_huella()`, `TipoFactura` enum |
| **修改** `shared/src/order/mod.rs` | 导出 verifactu 模块 |
| **新建** `shared/src/cloud/invoice_sync.rs` | `InvoiceSync` 类型 |
| **修改** `shared/src/cloud/sync.rs` | 添加 `SyncResource::Invoice`, `InvoiceSync` 导出 |
| **新建** `edge-server/src/archiving/invoice.rs` | `InvoiceService` |
| **修改** `edge-server/src/archiving/mod.rs` | 导出 invoice 模块 |
| **修改** `edge-server/src/archiving/service.rs` | archive 后调用 InvoiceService |
| **修改** `edge-server/src/archiving/credit_note.rs` | create 后调用 InvoiceService |
| **新建** `edge-server/migrations/000N_invoice.sql` | invoice + invoice_desglose + invoice_counter 表 |
| **修改** `edge-server/migrations/` (system_state) | ALTER ADD last_huella |
| **修改** `edge-server/src/db/repository/` | 新建 invoice repo |
| **修改** `edge-server/src/cloud/worker.rs` | 同步 invoice 到云端 |

## Phase 划分

### Phase 1 (本次实现)
- `invoice` / `invoice_desglose` / `invoice_counter` 表
- `compute_verifactu_huella()` in shared
- `InvoiceService` — F2 (Completed 订单) + R5 (Credit Note)
- 集成到 archive + credit_note 流程
- InvoiceSync 类型 + 云端同步
- system_state.last_huella

### Phase 2 (后续)
- F3 (升级完整发票)
- Anulación (先 Complete 再 Void 的场景)
- P12 证书 NIF 提取
- LossSettled 发票处理
- 云端 AEAT XML 组装 + 提交队列
