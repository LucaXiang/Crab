# Invoice Cloud Sync Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Edge-server 创建的 Verifactu 发票 (F2/R5) 同步到 cloud，cloud 负责 AEAT 提交并回传状态更新。

**Architecture:**
- Edge 是发票创建的权威（huella 链、发票号、金额）
- Cloud 是 AEAT 状态的权威（Submitted/Accepted/Rejected）
- 同步方向：Edge→Cloud（发票数据），Cloud→Edge（aeat_status 更新）
- P12 证书上传时已绑定租户，NIF 从 P12 的 serial_number 提取（`P12CertInfo::tax_id()`），不可更改

**Tech Stack:** Rust, SQLx (SQLite edge / PostgreSQL cloud), shared types

---

### Task 1: Edge — build_sync 函数和 list_unsynced_ids

**Files:**
- Modify: `edge-server/src/db/repository/invoice.rs`

**已有：** `list_unsynced(pool, limit)` → `Vec<Invoice>`, `get_desglose(pool, invoice_id)`, `mark_synced(pool, ids)`

**需要添加：**

**Step 1: 添加 list_unsynced_ids 函数**

```rust
/// List unsynced invoice IDs (for batch sync pattern compatibility).
pub async fn list_unsynced_ids(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<i64>> {
    let ids: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM invoice WHERE cloud_synced = 0 ORDER BY id LIMIT ?"
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(ids.into_iter().map(|r| r.0).collect())
}
```

**Step 2: 添加 build_sync 函数**

构建 `InvoiceSync` 结构体，包含 desglose 行。

```rust
use shared::cloud::sync::{InvoiceSync, TaxDesglose};
use rust_decimal::Decimal;

/// Build InvoiceSync payload for cloud sync.
pub async fn build_sync(pool: &SqlitePool, invoice_id: i64) -> RepoResult<InvoiceSync> {
    let invoice = get_by_id(pool, invoice_id).await?.ok_or_else(|| {
        super::RepoError::NotFound(format!("invoice {invoice_id}"))
    })?;
    let desglose_rows = get_desglose(pool, invoice_id).await?;
    let desglose: Vec<TaxDesglose> = desglose_rows.into_iter().map(|d| TaxDesglose {
        tax_rate: d.tax_rate as i32,
        base_amount: Decimal::try_from(d.base_amount).unwrap_or_default(),
        tax_amount: Decimal::try_from(d.tax_amount).unwrap_or_default(),
    }).collect();

    Ok(InvoiceSync {
        id: invoice.id,
        invoice_number: invoice.invoice_number,
        serie: invoice.serie,
        tipo_factura: invoice.tipo_factura,
        source_type: invoice.source_type,
        source_pk: invoice.source_pk,
        subtotal: invoice.subtotal,
        tax: invoice.tax,
        total: invoice.total,
        desglose,
        huella: invoice.huella,
        prev_huella: invoice.prev_huella,
        fecha_expedicion: invoice.fecha_expedicion,
        nif: invoice.nif,
        nombre_razon: invoice.nombre_razon,
        factura_rectificada_id: invoice.factura_rectificada_id,
        factura_rectificada_num: invoice.factura_rectificada_num,
        created_at: invoice.created_at,
    })
}
```

**Step 3: 添加 get_by_id 辅助函数**

```rust
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Invoice>> {
    let row = sqlx::query_as::<_, InvoiceRow>(&format!(
        "SELECT {INVOICE_COLUMNS} FROM invoice WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(InvoiceRow::into_invoice))
}
```

**Step 4: Commit**

```bash
git add edge-server/src/db/repository/invoice.rs
git commit -m "feat(verifactu): add invoice build_sync, list_unsynced_ids, get_by_id for cloud sync"
```

---

### Task 2: Edge — CloudWorker sync_invoices_http

**Files:**
- Modify: `edge-server/src/cloud/worker.rs`

**Step 1: 添加 sync_invoices_http 方法**

复制 `sync_credit_notes_http` 模式（lines 649-762），替换：
- `credit_note::list_unsynced_ids` → `invoice::list_unsynced_ids`
- `credit_note::build_sync` → `invoice::build_sync`
- `SyncResource::CreditNote` → `SyncResource::Invoice`
- `credit_note::mark_synced_batch` → `invoice::mark_synced`
- 日志中 "credit note" → "invoice"

```rust
async fn sync_invoices_http(&mut self) -> Result<(), crate::utils::AppError> {
    let binding = self.get_binding().await?;

    loop {
        let ids = invoice::list_unsynced_ids(&self.state.pool, ARCHIVED_ORDER_BATCH_SIZE)
            .await
            .map_err(|e| crate::utils::AppError::internal(format!("List unsynced invoices: {e}")))?;

        if ids.is_empty() { break; }

        let mut items: Vec<CloudSyncItem> = Vec::with_capacity(ids.len());
        let mut synced_ids: Vec<i64> = Vec::with_capacity(ids.len());
        let mut skipped_ids: Vec<i64> = Vec::new();

        for &id in &ids {
            match invoice::build_sync(&self.state.pool, id).await {
                Ok(inv_sync) => {
                    let data = match serde_json::to_value(&inv_sync) {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::error!(invoice_id = id, "Failed to serialize InvoiceSync, skipping: {e}");
                            skipped_ids.push(id);
                            continue;
                        }
                    };
                    items.push(CloudSyncItem {
                        resource: SyncResource::Invoice,
                        version: id as u64,
                        action: shared::cloud::SyncAction::Upsert,
                        resource_id: id.to_string(),
                        data,
                    });
                    synced_ids.push(id);
                }
                Err(e) => {
                    tracing::error!(invoice_id = id, "Failed to build InvoiceSync, skipping: {e}");
                    skipped_ids.push(id);
                }
            }
        }

        // Mark permanently failed invoices as synced to unblock the queue
        if !skipped_ids.is_empty() {
            tracing::warn!(count = skipped_ids.len(), "Skipped unbuildable invoices");
            if let Err(e) = invoice::mark_synced(&self.state.pool, &skipped_ids).await {
                tracing::error!("Failed to mark skipped invoices as synced: {e}");
            }
        }

        if items.is_empty() {
            if skipped_ids.is_empty() { break; }
            continue;
        }

        let batch_count = items.len();
        let batch = CloudSyncBatch {
            edge_id: self.cloud_service.edge_id().to_string(),
            items,
            sent_at: shared::util::now_millis(),
        };

        let response = self.cloud_service.push_batch(batch, &binding).await
            .map_err(|e| crate::utils::AppError::internal(format!("HTTP sync invoices: {e}")))?;

        if response.rejected > 0 {
            tracing::warn!(accepted = response.accepted, rejected = response.rejected, "Invoice sync has rejections");
            break;
        }

        if let Err(e) = invoice::mark_synced(&self.state.pool, &synced_ids).await {
            tracing::error!("Failed to mark invoices as cloud_synced: {e}");
            break;
        }

        tracing::info!(batch_size = batch_count, accepted = response.accepted, "Invoices synced via HTTP");

        if (synced_ids.len() as i64) < ARCHIVED_ORDER_BATCH_SIZE { break; }
    }
    Ok(())
}
```

**Step 2: 添加 import**

在 worker.rs 顶部 use 区域加入 `invoice` repository：
```rust
use crate::db::repository::invoice;
```

**Step 3: Hook 进主循环（两处）**

在 periodic tick handler（约 line 213）和 archive_notify handler（约 line 223）各加一行：
```rust
if let Err(e) = self.sync_invoices_http().await {
    tracing::warn!("Invoice sync failed: {e}");
}
```

**Step 4: Commit**

```bash
git add edge-server/src/cloud/worker.rs
git commit -m "feat(verifactu): add sync_invoices_http to CloudWorker"
```

---

### Task 3: Cloud — PostgreSQL migration + upsert_invoice

**Files:**
- Create: `crab-cloud/migrations/XXXX_store_invoices.up.sql`
- Create: `crab-cloud/migrations/XXXX_store_invoices.down.sql`
- Modify: `crab-cloud/src/db/sync_store.rs`

**Step 1: 创建 PG migration**

```sql
-- store_invoices: Verifactu invoices synced from edge-servers
CREATE TABLE store_invoices (
    id              BIGINT PRIMARY KEY,  -- snowflake from edge
    store_id        BIGINT NOT NULL REFERENCES stores(id),
    tenant_id       TEXT NOT NULL,
    invoice_number  TEXT NOT NULL,
    serie           TEXT NOT NULL,
    tipo_factura    TEXT NOT NULL,        -- F2 / R5
    source_type     TEXT NOT NULL,        -- ORDER / CREDIT_NOTE
    source_pk       BIGINT NOT NULL,      -- archived_order.source_id / credit_note.source_id
    subtotal        DOUBLE PRECISION NOT NULL,
    tax             DOUBLE PRECISION NOT NULL,
    total           DOUBLE PRECISION NOT NULL,
    huella          TEXT NOT NULL,
    prev_huella     TEXT,
    fecha_expedicion TEXT NOT NULL,
    nif             TEXT NOT NULL,
    nombre_razon    TEXT NOT NULL,
    factura_rectificada_id   BIGINT,
    factura_rectificada_num  TEXT,
    -- Cloud-authoritative fields (AEAT status)
    aeat_status     TEXT NOT NULL DEFAULT 'PENDING',
    aeat_csv        TEXT,                 -- CSV code from AEAT acceptance
    aeat_submitted_at BIGINT,
    aeat_response_at  BIGINT,
    -- Detail JSONB (desglose lines + full payload)
    detail          JSONB NOT NULL,
    source_id       BIGINT NOT NULL,      -- edge invoice.id
    synced_at       BIGINT NOT NULL,
    created_at      BIGINT NOT NULL,

    UNIQUE(store_id, invoice_number)
);

CREATE INDEX idx_store_invoices_tenant ON store_invoices(tenant_id);
CREATE INDEX idx_store_invoices_aeat ON store_invoices(aeat_status) WHERE aeat_status != 'ACCEPTED';
CREATE INDEX idx_store_invoices_source ON store_invoices(store_id, source_type, source_pk);
```

Down migration:
```sql
DROP TABLE IF EXISTS store_invoices;
```

**Step 2: 在 sync_store.rs 添加 upsert_invoice**

在 `upsert_resource` 函数的 match 中加入 Invoice arm（在 CreditNote 之后）：

```rust
SyncResource::Invoice => upsert_invoice(pool, store_id, tenant_id, item, now).await,
```

新增函数：
```rust
async fn upsert_invoice(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::InvoiceSync;

    let inv: InvoiceSync = serde_json::from_value(item.data.clone())?;
    let source_id: i64 = item.resource_id.parse()?;

    sqlx::query(
        r#"
        INSERT INTO store_invoices
            (id, store_id, tenant_id, invoice_number, serie, tipo_factura,
             source_type, source_pk, subtotal, tax, total,
             huella, prev_huella, fecha_expedicion, nif, nombre_razon,
             factura_rectificada_id, factura_rectificada_num,
             aeat_status, detail, source_id, synced_at, created_at)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)
        ON CONFLICT (store_id, invoice_number) DO UPDATE SET
            detail = EXCLUDED.detail,
            synced_at = EXCLUDED.synced_at
        "#,
    )
    .bind(shared::util::snowflake_id())
    .bind(store_id)
    .bind(tenant_id)
    .bind(&inv.invoice_number)
    .bind(&inv.serie)
    .bind(inv.tipo_factura.as_str())
    .bind(inv.source_type.as_str())
    .bind(inv.source_pk)
    .bind(inv.subtotal)
    .bind(inv.tax)
    .bind(inv.total)
    .bind(&inv.huella)
    .bind(&inv.prev_huella)
    .bind(&inv.fecha_expedicion)
    .bind(&inv.nif)
    .bind(&inv.nombre_razon)
    .bind(inv.factura_rectificada_id)
    .bind(&inv.factura_rectificada_num)
    .bind("PENDING")
    .bind(serde_json::to_value(&inv)?)
    .bind(source_id)
    .bind(now)
    .bind(inv.created_at)
    .execute(pool)
    .await?;

    Ok(())
}
```

**Step 3: Commit**

```bash
git add crab-cloud/migrations/ crab-cloud/src/db/sync_store.rs
git commit -m "feat(verifactu): add store_invoices table and cloud sync ingestion"
```

---

### Task 4: Cloud→Edge — AEAT 状态回传

**Files:**
- Modify: `shared/src/cloud/ws.rs` 或 `shared/src/cloud/store_op.rs` — 添加 AEAT status update 操作
- Modify: `edge-server/src/cloud/rpc_executor.rs` — 处理 AEAT status update
- Modify: `edge-server/src/db/repository/invoice.rs` — 添加 update_aeat_status

**Step 1: 在 StoreOp 中添加 UpdateInvoiceAeatStatus variant**

```rust
// shared/src/cloud/store_op.rs
UpdateInvoiceAeatStatus {
    invoice_number: String,
    aeat_status: String,   // "SUBMITTED" | "ACCEPTED" | "REJECTED"
    aeat_csv: Option<String>,
}
```

**Step 2: Edge repository — update_aeat_status**

```rust
// edge-server/src/db/repository/invoice.rs
pub async fn update_aeat_status(
    pool: &SqlitePool,
    invoice_number: &str,
    aeat_status: AeatStatus,
    aeat_csv: Option<&str>,
) -> RepoResult<bool> {
    let result = sqlx::query(
        "UPDATE invoice SET aeat_status = ?1 WHERE invoice_number = ?2"
    )
    .bind(aeat_status.as_str())
    .bind(invoice_number)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
```

**Step 3: RPC executor — 处理 UpdateInvoiceAeatStatus**

在 `rpc_executor.rs` 的 match arm 中添加处理。

**Step 4: Commit**

```bash
git add shared/src/cloud/store_op.rs edge-server/src/db/repository/invoice.rs edge-server/src/cloud/rpc_executor.rs
git commit -m "feat(verifactu): add cloud→edge AEAT status update via StoreOp RPC"
```

---

### Task 5: Edge — NIF 从 P12 自动读取

**当前状态：** `state.rs` 从 `store_info.nif` 读取 NIF。
**目标：** NIF 应该优先从 P12 证书元数据获取（`P12CertInfo::tax_id()`），P12 上传时绑定到租户不可更改。

**Files:**
- Modify: `edge-server/src/core/state.rs` — InvoiceService 初始化时使用 P12 证书的 NIF
- 可能需要: `edge-server/src/services/tenant_binding.rs` 查看 credential 中是否已有 NIF

**设计决策：**
- Cloud 在处理 P12 上传时提取 NIF 并存入 `stores.nif` 或发下来
- Edge 通过 StoreInfo 已经能获取到 NIF（store_info.nif 由 cloud provisioning 设置）
- **结论：** NIF 来源链是 P12 → cloud 提取 → sync 到 store_info.nif → edge 读取。无需 edge 本地解析 P12。只需确保 cloud 在 P12 上传时自动填充 store_info.nif。

**Step 1: 确认 cloud 的 P12 上传流程已设置 NIF**

检查 `crab-cloud` 的 P12 上传 handler 是否将 `tax_id()` 写入 stores 表。

**Step 2: 如果没有，添加逻辑**

```rust
// P12 上传成功后
if let Some(nif) = cert_info.tax_id() {
    // 更新 stores.nif (或 store_info 同步字段)
    sqlx::query("UPDATE stores SET nif = $1 WHERE tenant_id = $2")
        .bind(nif)
        .bind(tenant_id)
        .execute(pool)
        .await?;
}
```

**Step 3: Commit**

```bash
git add crab-cloud/src/...
git commit -m "feat(verifactu): auto-populate store NIF from P12 certificate on upload"
```

---

### Task 6: InvoiceSync 添加 hash 验证（可选但推荐）

**Files:**
- Modify: `shared/src/cloud/sync.rs` — 添加 InvoiceSync 的 verify_huella 方法

**Step 1: 添加验证方法**

仿照 `CreditNoteSync::verify_hash()`，添加 `InvoiceSync::verify_huella()` 来让 cloud 在收到 invoice 时验证 huella 完整性。

```rust
impl InvoiceSync {
    /// Recompute huella and return Some(recomputed) if mismatch, None if ok.
    pub fn verify_huella(&self) -> Option<String> {
        use crate::order::verifactu::{compute_verifactu_huella_alta, HuellaAltaInput};
        let input = HuellaAltaInput {
            nif: &self.nif,
            invoice_number: &self.invoice_number,
            fecha_expedicion: &self.fecha_expedicion,
            tipo_factura: self.tipo_factura.as_str(),
            cuota_total: self.tax,
            importe_total: self.total,
            prev_huella: self.prev_huella.as_deref(),
            fecha_hora_registro: ???, // 问题：InvoiceSync 没有 fecha_hora_registro
        };
        // ...
    }
}
```

**注意：** `InvoiceSync` 当前缺少 `fecha_hora_registro` 字段。需要添加到 `InvoiceSync` 和 `Invoice` model 中（或者用 `created_at` ISO 转换）。

**决策点：是否在 Invoice 表中存储 fecha_hora_registro？**
- 选项 A：添加 `fecha_hora_registro TEXT` 列到 invoice 表 + InvoiceSync（推荐，完整性）
- 选项 B：从 `created_at` 推导（有风险，时区可能不一致）

**Step 2: 添加 fecha_hora_registro 字段到 Invoice**

如选择 A：migration ALTER TABLE + model 更新 + InvoiceSync 字段 + 验证函数。

**Step 3: 添加测试**

**Step 4: Commit**

---

### Task 7: 前端 — Invoice 卡片渲染 (HistoryDetail)

**Files:**
- Create: `red_coral/src/core/domain/types/invoice.ts` — Invoice 类型定义
- Modify: `red_coral/src/screens/History/HistoryDetail.tsx` — 添加 InvoiceSection
- Create: `red_coral/src/screens/History/InvoiceSection.tsx` — 发票卡片组件
- Modify: `red_coral/src/infrastructure/i18n/locales/zh-CN.json` — i18n keys
- Modify: `red_coral/src/infrastructure/i18n/locales/es-ES.json` — i18n keys
- Modify: `red_coral/src/infrastructure/i18n/locales/en-US.json` — i18n keys

**设计:**
- 在 HistoryDetail 的 CreditNoteSection 下方添加 InvoiceSection
- 通过 `invokeApi('fetch_invoices_by_order', { orderPk })` 获取关联发票（F2 + R5）
- 每张发票显示：tipo_factura badge、invoice_number、total、aeat_status badge
- R5 发票额外显示 `factura_rectificada_num`（被修正的原始发票号）
- aeat_status 颜色：Pending=gray, Submitted=blue, Accepted=green, Rejected=red

**Step 1: 创建 Invoice TypeScript 类型**

```typescript
export interface InvoiceSummary {
  id: number;
  invoice_number: string;
  serie: string;
  tipo_factura: string; // "F2" | "R5"
  source_type: string;  // "ORDER" | "CREDIT_NOTE"
  source_pk: number;
  subtotal: number;
  tax: number;
  total: number;
  aeat_status: string;  // "PENDING" | "SUBMITTED" | "ACCEPTED" | "REJECTED"
  factura_rectificada_num: string | null;
  created_at: number;
}
```

**Step 2: 创建 InvoiceSection 组件**

**Step 3: 在 HistoryDetail 中引入 InvoiceSection**

**Step 4: 添加 i18n keys**

**Step 5: 添加 Tauri 命令和 API handler**

**Step 6: Commit**

---

## 执行顺序

Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6 → Task 7

Task 1-2 是 edge 侧，可以先做完并测试。
Task 3 是 cloud 侧 ingestion。
Task 4 是双向闭环。
Task 5 是 NIF 来源统一。
Task 6 是 huella 验证增强。
Task 7 是前端 invoice 渲染（依赖 edge-server API）。

## 关键约束

- **Edge 是发票创建的唯一权威** — cloud 不创建/修改发票内容
- **Cloud 是 AEAT 状态的唯一权威** — edge 不修改 aeat_status
- **P12 绑定租户不可更改** — NIF 从 P12 提取，一次性写入
- **Invoice 通过 source_type + source_pk 关联 order/credit_note** — UI 可在订单详情中渲染发票卡片
- **huella 链由 edge 维护** — cloud 只验证不修改
