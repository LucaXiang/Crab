# Verifactu Local Infrastructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Verifactu-compliant invoice generation (F2/R5), huella hash chain, and invoice numbering on edge-server, so cloud has everything needed for AEAT submission.

**Architecture:** Invoice records are created atomically inside existing archive/credit_note transactions. A new `InvoiceService` manages numbering (Serie+date counter) and huella computation (Verifactu key=value& SHA-256). Two independent hash chains coexist: internal `chain_entry` (data integrity) and Verifactu `huella` (tax compliance).

**Tech Stack:** Rust, SQLite (sqlx), SHA-256, rust_decimal, chrono/chrono-tz

**Design doc:** `docs/plans/2026-02-27-verifactu-local-design.md`

---

### Task 1: Verifactu Huella Computation (shared)

**Files:**
- Create: `shared/src/order/verifactu.rs`
- Modify: `shared/src/order/mod.rs:8-26`

**Step 1: Write the failing test**

```rust
// shared/src/order/verifactu.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_amount_removes_trailing_zeros() {
        assert_eq!(format_amount(123.10), "123.1");
        assert_eq!(format_amount(100.00), "100");
        assert_eq!(format_amount(99.99), "99.99");
        assert_eq!(format_amount(0.0), "0");
    }

    #[test]
    fn test_huella_alta_deterministic() {
        let h1 = compute_verifactu_huella_alta(
            "B12345678",
            "A-20260227-0001",
            "27-02-2026",
            "F2",
            2.1,
            23.1,
            None,
            "2026-02-27T10:30:00+01:00",
        );
        let h2 = compute_verifactu_huella_alta(
            "B12345678",
            "A-20260227-0001",
            "27-02-2026",
            "F2",
            2.1,
            23.1,
            None,
            "2026-02-27T10:30:00+01:00",
        );
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_huella_alta_first_record_empty_prev() {
        // First record: Huella= (empty value)
        let h = compute_verifactu_huella_alta(
            "B12345678",
            "A-20260227-0001",
            "27-02-2026",
            "F2",
            2.1,
            23.1,
            None,
            "2026-02-27T10:30:00+01:00",
        );
        // Verify it uses "Huella=" (empty) not "Huella=genesis" or similar
        // We verify by computing manually
        let input = "IDEmisorFactura=B12345678&NumSerieFactura=A-20260227-0001&\
            FechaExpedicionFactura=27-02-2026&TipoFactura=F2&\
            CuotaTotal=2.1&ImporteTotal=23.1&\
            Huella=&FechaHoraHusoGenRegistro=2026-02-27T10:30:00+01:00";
        use sha2::{Digest, Sha256};
        let expected = format!("{:x}", Sha256::digest(input.as_bytes()));
        assert_eq!(h, expected);
    }

    #[test]
    fn test_huella_alta_with_prev() {
        let prev = "abc123def456";
        let h = compute_verifactu_huella_alta(
            "B12345678",
            "A-20260227-0002",
            "27-02-2026",
            "R5",
            1.0,
            11.0,
            Some(prev),
            "2026-02-27T10:31:00+01:00",
        );
        // Chained record includes prev huella
        let input = format!(
            "IDEmisorFactura=B12345678&NumSerieFactura=A-20260227-0002&\
             FechaExpedicionFactura=27-02-2026&TipoFactura=R5&\
             CuotaTotal=1&ImporteTotal=11&\
             Huella={prev}&FechaHoraHusoGenRegistro=2026-02-27T10:31:00+01:00"
        );
        use sha2::{Digest, Sha256};
        let expected = format!("{:x}", Sha256::digest(input.as_bytes()));
        assert_eq!(h, expected);
    }

    #[test]
    fn test_huella_different_amount_different_hash() {
        let h1 = compute_verifactu_huella_alta(
            "B12345678", "A-20260227-0001", "27-02-2026", "F2",
            2.1, 23.1, None, "2026-02-27T10:30:00+01:00",
        );
        let h2 = compute_verifactu_huella_alta(
            "B12345678", "A-20260227-0001", "27-02-2026", "F2",
            2.2, 23.2, None, "2026-02-27T10:30:00+01:00",
        );
        assert_ne!(h1, h2);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p shared --lib order::verifactu`
Expected: FAIL — module not found

**Step 3: Write implementation**

```rust
// shared/src/order/verifactu.rs

//! Verifactu huella (hash chain) computation
//!
//! Implements AEAT's Registro de Alta hash format:
//! `key=value&` concatenation → UTF-8 → SHA-256 → 64-char hex
//!
//! Reference: https://sede.agenciatributaria.gob.es/Sede/iva/sistemas-informaticos-facturacion-verifactu/

use sha2::{Digest, Sha256};

/// Verifactu Registro de Alta huella (F2, R5 invoices)
///
/// Fields (AEAT spec order):
/// 1. IDEmisorFactura (NIF)
/// 2. NumSerieFactura (invoice number)
/// 3. FechaExpedicionFactura (DD-MM-YYYY)
/// 4. TipoFactura (F2, R5, etc.)
/// 5. CuotaTotal (total tax)
/// 6. ImporteTotal (total with tax)
/// 7. Huella (previous huella or empty)
/// 8. FechaHoraHusoGenRegistro (ISO 8601 with TZ)
pub fn compute_verifactu_huella_alta(
    nif: &str,
    invoice_number: &str,
    fecha_expedicion: &str,
    tipo_factura: &str,
    cuota_total: f64,
    importe_total: f64,
    prev_huella: Option<&str>,
    fecha_hora_registro: &str,
) -> String {
    let input = format!(
        "IDEmisorFactura={}&NumSerieFactura={}&\
         FechaExpedicionFactura={}&TipoFactura={}&\
         CuotaTotal={}&ImporteTotal={}&\
         Huella={}&FechaHoraHusoGenRegistro={}",
        nif,
        invoice_number,
        fecha_expedicion,
        tipo_factura,
        format_amount(cuota_total),
        format_amount(importe_total),
        prev_huella.unwrap_or(""),
        fecha_hora_registro,
    );

    format!("{:x}", Sha256::digest(input.as_bytes()))
}

/// Format amount for Verifactu: remove trailing zeros
/// 123.10 → "123.1", 100.00 → "100", 99.99 → "99.99"
fn format_amount(v: f64) -> String {
    // Use rust_decimal for exact representation
    use rust_decimal::prelude::*;
    let d = Decimal::from_f64(v).unwrap_or_default();
    let s = d.normalize().to_string();
    s
}

// Tests at bottom (see Step 1)
```

**Step 4: Register module**

In `shared/src/order/mod.rs`, add:
```rust
pub mod verifactu;
pub use verifactu::compute_verifactu_huella_alta;
```

**Step 5: Run test to verify it passes**

Run: `cargo test -p shared --lib order::verifactu`
Expected: all 5 tests PASS

**Step 6: Commit**

```bash
git add shared/src/order/verifactu.rs shared/src/order/mod.rs
git commit -m "feat(verifactu): add huella hash computation per AEAT spec"
```

---

### Task 2: Invoice Model (shared)

**Files:**
- Create: `shared/src/models/invoice.rs`
- Modify: `shared/src/models/mod.rs:7-52`

**Step 1: Write invoice model**

```rust
// shared/src/models/invoice.rs

//! Invoice model for Verifactu compliance

use serde::{Deserialize, Serialize};

/// Verifactu invoice types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoFactura {
    /// Factura Simplificada (normal POS receipt)
    F2,
    /// Factura Rectificativa en Simplificadas (credit note)
    R5,
}

impl TipoFactura {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::F2 => "F2",
            Self::R5 => "R5",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "F2" => Some(Self::F2),
            "R5" => Some(Self::R5),
            _ => None,
        }
    }
}

impl std::fmt::Display for TipoFactura {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Invoice source type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceSourceType {
    Order,
    CreditNote,
}

impl InvoiceSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Order => "ORDER",
            Self::CreditNote => "CREDIT_NOTE",
        }
    }
}

/// AEAT submission status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AeatStatus {
    Pending,
    Submitted,
    Accepted,
    Rejected,
}

impl AeatStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Submitted => "SUBMITTED",
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
        }
    }
}

/// Invoice record (Verifactu compliant)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Invoice {
    pub id: i64,
    pub invoice_number: String,
    pub serie: String,
    pub tipo_factura: String,
    pub source_type: String,
    pub source_pk: i64,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub huella: String,
    pub prev_huella: Option<String>,
    pub fecha_expedicion: String,
    pub nif: String,
    pub nombre_razon: String,
    pub factura_rectificada_id: Option<i64>,
    pub factura_rectificada_num: Option<String>,
    pub cloud_synced: bool,
    pub aeat_status: String,
    pub created_at: i64,
}

/// Invoice desglose (tax breakdown line)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct InvoiceDesglose {
    pub id: i64,
    pub invoice_id: i64,
    pub tax_rate: i64,       // basis points (1000 = 10.00%)
    pub base_amount: f64,
    pub tax_amount: f64,
}
```

**Step 2: Register module**

In `shared/src/models/mod.rs`, add:
```rust
pub mod invoice;
pub use invoice::*;
```

**Step 3: Verify compilation**

Run: `cargo check -p shared`
Expected: OK

**Step 4: Commit**

```bash
git add shared/src/models/invoice.rs shared/src/models/mod.rs
git commit -m "feat(verifactu): add Invoice and InvoiceDesglose models"
```

---

### Task 3: InvoiceSync Type (shared cloud)

**Files:**
- Modify: `shared/src/cloud/sync.rs`

**Step 1: Add InvoiceSync and SyncResource::Invoice**

In `shared/src/cloud/sync.rs`, add `Invoice` variant to `SyncResource` enum (after `CreditNote`):
```rust
/// Verifactu invoices (edge → cloud only)
Invoice,
```

Update `as_str()` match:
```rust
Self::Invoice => "invoice",
```

Add InvoiceSync struct (near other sync types like `OrderDetailSync`, `CreditNoteSync`):

```rust
/// Invoice data synced to cloud for Verifactu AEAT submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSync {
    pub id: i64,
    pub invoice_number: String,
    pub serie: String,
    pub tipo_factura: String,
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

**Step 2: Verify compilation**

Run: `cargo check --workspace`
Expected: OK (may need to handle exhaustive match in cloud worker)

**Step 3: Commit**

```bash
git add shared/src/cloud/sync.rs
git commit -m "feat(verifactu): add InvoiceSync type and SyncResource::Invoice"
```

---

### Task 4: SQLite Migration

**Files:**
- Create: `edge-server/migrations/0005_invoice.up.sql`
- Create: `edge-server/migrations/0005_invoice.down.sql`

**Step 1: Write up migration**

```sql
-- edge-server/migrations/0005_invoice.up.sql

-- Verifactu invoice table
CREATE TABLE invoice (
    id              INTEGER PRIMARY KEY,
    invoice_number  TEXT NOT NULL UNIQUE,
    serie           TEXT NOT NULL,
    tipo_factura    TEXT NOT NULL,
    source_type     TEXT NOT NULL,
    source_pk       INTEGER NOT NULL,
    subtotal        REAL NOT NULL,
    tax             REAL NOT NULL,
    total           REAL NOT NULL,
    huella          TEXT NOT NULL,
    prev_huella     TEXT,
    fecha_expedicion TEXT NOT NULL,
    nif             TEXT NOT NULL,
    nombre_razon    TEXT NOT NULL,
    factura_rectificada_id  INTEGER,
    factura_rectificada_num TEXT,
    cloud_synced    INTEGER NOT NULL DEFAULT 0,
    aeat_status     TEXT NOT NULL DEFAULT 'PENDING',
    created_at      INTEGER NOT NULL
);

CREATE INDEX idx_invoice_source ON invoice(source_type, source_pk);
CREATE INDEX idx_invoice_cloud_synced ON invoice(cloud_synced);
CREATE INDEX idx_invoice_serie_number ON invoice(serie, invoice_number);

-- Invoice tax breakdown (desglose)
CREATE TABLE invoice_desglose (
    id          INTEGER PRIMARY KEY,
    invoice_id  INTEGER NOT NULL REFERENCES invoice(id),
    tax_rate    INTEGER NOT NULL,
    base_amount REAL NOT NULL,
    tax_amount  REAL NOT NULL,
    UNIQUE(invoice_id, tax_rate)
);

-- Invoice counter (crash-safe numbering per Serie)
CREATE TABLE invoice_counter (
    serie       TEXT PRIMARY KEY,
    date_str    TEXT NOT NULL,
    last_number INTEGER NOT NULL
);

-- Add Verifactu huella chain to system_state
ALTER TABLE system_state ADD COLUMN last_huella TEXT;
```

**Step 2: Write down migration**

```sql
-- edge-server/migrations/0005_invoice.down.sql

DROP TABLE IF EXISTS invoice_desglose;
DROP TABLE IF EXISTS invoice_counter;
DROP TABLE IF EXISTS invoice;
-- SQLite cannot ALTER TABLE DROP COLUMN in older versions,
-- but sqlx handles this via table recreation if needed.
-- For dev phase, this is acceptable.
```

**Step 3: Run migration**

Run: `sqlx migrate run --source edge-server/migrations`
Expected: Applied 0005_invoice

**Step 4: Update SystemState model**

In `shared/src/models/system_state.rs`, add to `SystemState`:
```rust
pub last_huella: Option<String>,
```

Update `SystemStateUpdate`:
```rust
pub last_huella: Option<String>,
```

Update `edge-server/src/db/repository/system_state.rs`:
- Add `last_huella` to the SELECT in `get()`
- Add `last_huella = COALESCE(?N, last_huella)` to the UPDATE in `update()`

**Step 5: Verify compilation**

Run: `cargo check --workspace`
Expected: OK (fix any exhaustive match issues)

**Step 6: Commit**

```bash
git add edge-server/migrations/0005_invoice.up.sql edge-server/migrations/0005_invoice.down.sql \
        shared/src/models/system_state.rs edge-server/src/db/repository/system_state.rs
git commit -m "feat(verifactu): add invoice tables migration and SystemState.last_huella"
```

---

### Task 5: Invoice Repository (edge-server)

**Files:**
- Create: `edge-server/src/db/repository/invoice.rs`
- Modify: `edge-server/src/db/repository/mod.rs:23-34`

**Step 1: Write invoice repository**

```rust
// edge-server/src/db/repository/invoice.rs

//! Invoice repository — CRUD for Verifactu invoices

use shared::models::{Invoice, InvoiceDesglose};
use sqlx::SqlitePool;

use super::{RepoError, RepoResult};

/// Insert invoice and return the assigned id
pub async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    invoice: &Invoice,
) -> RepoResult<i64> {
    let id = shared::util::snowflake_id();
    sqlx::query(
        "INSERT INTO invoice \
         (id, invoice_number, serie, tipo_factura, source_type, source_pk, \
          subtotal, tax, total, huella, prev_huella, fecha_expedicion, \
          nif, nombre_razon, factura_rectificada_id, factura_rectificada_num, \
          cloud_synced, aeat_status, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,0,?17,?18)",
    )
    .bind(id)
    .bind(&invoice.invoice_number)
    .bind(&invoice.serie)
    .bind(&invoice.tipo_factura)
    .bind(&invoice.source_type)
    .bind(invoice.source_pk)
    .bind(invoice.subtotal)
    .bind(invoice.tax)
    .bind(invoice.total)
    .bind(&invoice.huella)
    .bind(&invoice.prev_huella)
    .bind(&invoice.fecha_expedicion)
    .bind(&invoice.nif)
    .bind(&invoice.nombre_razon)
    .bind(invoice.factura_rectificada_id)
    .bind(&invoice.factura_rectificada_num)
    .bind(&invoice.aeat_status)
    .bind(invoice.created_at)
    .execute(&mut **tx)
    .await?;
    Ok(id)
}

/// Insert invoice desglose line
pub async fn insert_desglose(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    invoice_id: i64,
    tax_rate: i64,
    base_amount: f64,
    tax_amount: f64,
) -> RepoResult<()> {
    let id = shared::util::snowflake_id();
    sqlx::query(
        "INSERT INTO invoice_desglose (id, invoice_id, tax_rate, base_amount, tax_amount) \
         VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(id)
    .bind(invoice_id)
    .bind(tax_rate)
    .bind(base_amount)
    .bind(tax_amount)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Get next invoice number for a Serie on a given date.
/// Uses invoice_counter table with double-check against actual invoices.
pub async fn next_invoice_number(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    serie: &str,
    date_str: &str, // YYYYMMDD
) -> RepoResult<String> {
    // Read current counter
    let row = sqlx::query_as::<_, CounterRow>(
        "SELECT date_str, last_number FROM invoice_counter WHERE serie = ?",
    )
    .bind(serie)
    .fetch_optional(&mut **tx)
    .await?;

    let next = match row {
        Some(r) if r.date_str == date_str => r.last_number + 1,
        _ => {
            // New day or first ever — double-check against actual invoices
            let max_from_invoices: Option<i64> = sqlx::query_scalar(
                "SELECT MAX(CAST(SUBSTR(invoice_number, -4) AS INTEGER)) \
                 FROM invoice WHERE serie = ? AND invoice_number LIKE ?",
            )
            .bind(serie)
            .bind(format!("{}-{}-%%", serie, date_str))
            .fetch_one(&mut **tx)
            .await?;
            max_from_invoices.unwrap_or(0) + 1
        }
    };

    // Upsert counter
    sqlx::query(
        "INSERT INTO invoice_counter (serie, date_str, last_number) VALUES (?1,?2,?3) \
         ON CONFLICT(serie) DO UPDATE SET date_str = ?2, last_number = ?3",
    )
    .bind(serie)
    .bind(date_str)
    .bind(next)
    .execute(&mut **tx)
    .await?;

    Ok(format!("{}-{}-{:04}", serie, date_str, next))
}

/// List unsynced invoices for cloud push
pub async fn list_unsynced(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<Invoice>> {
    let rows = sqlx::query_as::<_, Invoice>(
        "SELECT id, invoice_number, serie, tipo_factura, source_type, source_pk, \
         subtotal, tax, total, huella, prev_huella, fecha_expedicion, \
         nif, nombre_razon, factura_rectificada_id, factura_rectificada_num, \
         cloud_synced, aeat_status, created_at \
         FROM invoice WHERE cloud_synced = 0 ORDER BY id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get desglose for an invoice
pub async fn get_desglose(pool: &SqlitePool, invoice_id: i64) -> RepoResult<Vec<InvoiceDesglose>> {
    let rows = sqlx::query_as::<_, InvoiceDesglose>(
        "SELECT id, invoice_id, tax_rate, base_amount, tax_amount \
         FROM invoice_desglose WHERE invoice_id = ?",
    )
    .bind(invoice_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Mark invoices as synced
pub async fn mark_synced(pool: &SqlitePool, ids: &[i64]) -> RepoResult<()> {
    for id in ids {
        sqlx::query("UPDATE invoice SET cloud_synced = 1 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Find F2 invoice for an order (for R5 reference)
pub async fn find_order_invoice(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    order_pk: i64,
) -> RepoResult<Option<Invoice>> {
    let row = sqlx::query_as::<_, Invoice>(
        "SELECT id, invoice_number, serie, tipo_factura, source_type, source_pk, \
         subtotal, tax, total, huella, prev_huella, fecha_expedicion, \
         nif, nombre_razon, factura_rectificada_id, factura_rectificada_num, \
         cloud_synced, aeat_status, created_at \
         FROM invoice WHERE source_type = 'ORDER' AND source_pk = ? AND tipo_factura = 'F2'",
    )
    .bind(order_pk)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row)
}

#[derive(Debug, sqlx::FromRow)]
struct CounterRow {
    date_str: String,
    last_number: i64,
}
```

**Step 2: Register module**

In `edge-server/src/db/repository/mod.rs`, add:
```rust
// Invoice (Verifactu)
pub mod invoice;
```

**Step 3: Verify compilation**

Run: `cargo check -p edge-server`
Expected: OK

**Step 4: Commit**

```bash
git add edge-server/src/db/repository/invoice.rs edge-server/src/db/repository/mod.rs
git commit -m "feat(verifactu): add invoice repository with counter and desglose"
```

---

### Task 6: InvoiceService (edge-server)

**Files:**
- Create: `edge-server/src/archiving/invoice.rs`
- Modify: `edge-server/src/archiving/mod.rs:1-19`

**Step 1: Write InvoiceService**

```rust
// edge-server/src/archiving/invoice.rs

//! Invoice Service — generates Verifactu-compliant invoices
//!
//! Creates F2 (normal orders) and R5 (credit notes) invoices with
//! huella hash chain integrity. Called atomically within archive/credit_note
//! transactions — shares the same hash_chain_lock.

use crate::db::repository::{invoice as inv_repo, system_state};
use shared::cloud::sync::TaxDesglose;
use shared::models::Invoice;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::service::{ArchiveError, ArchiveResult};

/// Service for creating Verifactu invoices
#[derive(Clone)]
pub struct InvoiceService {
    pool: SqlitePool,
    tz: chrono_tz::Tz,
    /// Serie letter derived from store_number (1→A, 2→B, ...)
    serie: String,
    /// NIF from StoreInfo (Phase 1) or P12 (Phase 2)
    nif: String,
    /// Business name
    nombre_razon: String,
}

impl InvoiceService {
    pub fn new(
        pool: SqlitePool,
        tz: chrono_tz::Tz,
        store_number: u32,
        nif: String,
        nombre_razon: String,
    ) -> Self {
        let serie = store_number_to_serie(store_number);
        Self {
            pool,
            tz,
            serie,
            nif,
            nombre_razon,
        }
    }

    /// Create F2 invoice for a completed order.
    /// Called INSIDE archive_order transaction (tx already open, hash_chain_lock held).
    ///
    /// Returns None if order total is 0 (comped, no invoice needed).
    pub async fn create_order_invoice(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        order_pk: i64,
        subtotal: f64,
        tax: f64,
        total: f64,
        desglose: &[TaxDesglose],
    ) -> ArchiveResult<Option<i64>> {
        // Comped orders (total=0) don't get invoices
        if total <= 0.0 {
            return Ok(None);
        }

        let now = shared::util::now_millis();
        let now_dt = chrono::Utc::now().with_timezone(&self.tz);
        let date_str = now_dt.format("%Y%m%d").to_string();
        let fecha_expedicion = now_dt.format("%d-%m-%Y").to_string();
        let fecha_hora_registro = now_dt.to_rfc3339();

        // Get previous huella from system_state
        let system_state = system_state::get_or_create(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        let prev_huella = system_state.last_huella;

        // Allocate invoice number
        let invoice_number = inv_repo::next_invoice_number(tx, &self.serie, &date_str)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // Compute huella
        let huella = shared::order::compute_verifactu_huella_alta(
            &self.nif,
            &invoice_number,
            &fecha_expedicion,
            "F2",
            tax,
            total,
            prev_huella.as_deref(),
            &fecha_hora_registro,
        );

        // Insert invoice
        let invoice = Invoice {
            id: 0, // will be assigned
            invoice_number: invoice_number.clone(),
            serie: self.serie.clone(),
            tipo_factura: "F2".to_string(),
            source_type: "ORDER".to_string(),
            source_pk: order_pk,
            subtotal,
            tax,
            total,
            huella: huella.clone(),
            prev_huella: prev_huella.clone(),
            fecha_expedicion,
            nif: self.nif.clone(),
            nombre_razon: self.nombre_razon.clone(),
            factura_rectificada_id: None,
            factura_rectificada_num: None,
            cloud_synced: false,
            aeat_status: "PENDING".to_string(),
            created_at: now,
        };

        let invoice_id = inv_repo::insert(tx, &invoice)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // Insert desglose lines
        for d in desglose {
            inv_repo::insert_desglose(tx, invoice_id, d.tax_rate, d.base_amount, d.tax_amount)
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        // Update system_state.last_huella
        sqlx::query("UPDATE system_state SET last_huella = ?1, updated_at = ?2 WHERE id = 1")
            .bind(&huella)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            invoice_number = %invoice_number,
            tipo = "F2",
            total = total,
            "Invoice created for order"
        );

        Ok(Some(invoice_id))
    }

    /// Create R5 invoice for a credit note.
    /// Called INSIDE create_credit_note transaction.
    pub async fn create_credit_note_invoice(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        credit_note_pk: i64,
        original_order_pk: i64,
        subtotal_credit: f64,
        tax_credit: f64,
        total_credit: f64,
        desglose: &[TaxDesglose],
    ) -> ArchiveResult<Option<i64>> {
        let now = shared::util::now_millis();
        let now_dt = chrono::Utc::now().with_timezone(&self.tz);
        let date_str = now_dt.format("%Y%m%d").to_string();
        let fecha_expedicion = now_dt.format("%d-%m-%Y").to_string();
        let fecha_hora_registro = now_dt.to_rfc3339();

        // Find the original F2 invoice
        let original_f2 = inv_repo::find_order_invoice(tx, original_order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let (rectificada_id, rectificada_num) = match &original_f2 {
            Some(f2) => (Some(f2.id), Some(f2.invoice_number.clone())),
            None => {
                tracing::warn!(
                    original_order_pk = original_order_pk,
                    "No F2 invoice found for credit note — order may have been comped"
                );
                (None, None)
            }
        };

        // Get previous huella
        let system_state = system_state::get_or_create(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        let prev_huella = system_state.last_huella;

        // Allocate invoice number (same Serie + sequence as F2)
        let invoice_number = inv_repo::next_invoice_number(tx, &self.serie, &date_str)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // Compute huella
        let huella = shared::order::compute_verifactu_huella_alta(
            &self.nif,
            &invoice_number,
            &fecha_expedicion,
            "R5",
            tax_credit,
            total_credit,
            prev_huella.as_deref(),
            &fecha_hora_registro,
        );

        let invoice = Invoice {
            id: 0,
            invoice_number: invoice_number.clone(),
            serie: self.serie.clone(),
            tipo_factura: "R5".to_string(),
            source_type: "CREDIT_NOTE".to_string(),
            source_pk: credit_note_pk,
            subtotal: subtotal_credit,
            tax: tax_credit,
            total: total_credit,
            huella: huella.clone(),
            prev_huella: prev_huella.clone(),
            fecha_expedicion,
            nif: self.nif.clone(),
            nombre_razon: self.nombre_razon.clone(),
            factura_rectificada_id: rectificada_id,
            factura_rectificada_num: rectificada_num,
            cloud_synced: false,
            aeat_status: "PENDING".to_string(),
            created_at: now,
        };

        let invoice_id = inv_repo::insert(tx, &invoice)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        for d in desglose {
            inv_repo::insert_desglose(tx, invoice_id, d.tax_rate, d.base_amount, d.tax_amount)
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        // Update system_state.last_huella
        sqlx::query("UPDATE system_state SET last_huella = ?1, updated_at = ?2 WHERE id = 1")
            .bind(&huella)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            invoice_number = %invoice_number,
            tipo = "R5",
            total = total_credit,
            "Invoice created for credit note"
        );

        Ok(Some(invoice_id))
    }
}

/// Convert store_number to Serie letter: 1→A, 2→B, ..., 26→Z
fn store_number_to_serie(n: u32) -> String {
    if n == 0 || n > 26 {
        return format!("S{}", n);
    }
    String::from(char::from(b'A' + (n - 1) as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_number_to_serie() {
        assert_eq!(store_number_to_serie(1), "A");
        assert_eq!(store_number_to_serie(2), "B");
        assert_eq!(store_number_to_serie(26), "Z");
        assert_eq!(store_number_to_serie(0), "S0");
        assert_eq!(store_number_to_serie(27), "S27");
    }
}
```

**Step 2: Register module**

In `edge-server/src/archiving/mod.rs`, add:
```rust
pub mod invoice;
pub use invoice::InvoiceService;
```

**Step 3: Verify compilation**

Run: `cargo check -p edge-server`
Expected: OK

**Step 4: Commit**

```bash
git add edge-server/src/archiving/invoice.rs edge-server/src/archiving/mod.rs
git commit -m "feat(verifactu): add InvoiceService with F2/R5 creation"
```

---

### Task 7: Integrate into Archive Flow

**Files:**
- Modify: `edge-server/src/archiving/service.rs:155-178` (OrderArchiveService fields + constructor)
- Modify: `edge-server/src/archiving/service.rs:675-702` (archive_order — add invoice hook)

**Step 1: Add InvoiceService to OrderArchiveService**

Add field `invoice_service: Option<InvoiceService>` to `OrderArchiveService`. Make it `Option` so existing code doesn't break before InvoiceService is initialized (NIF/store_number might not be available on first start).

In constructor, accept `Option<InvoiceService>`:
```rust
pub fn new(
    pool: SqlitePool,
    tz: chrono_tz::Tz,
    data_dir: &std::path::Path,
    invoice_service: Option<InvoiceService>,
) -> Self {
    // ... existing ...
    Self { ..., invoice_service }
}
```

**Step 2: Add invoice creation after chain_entry in archive_order**

After step 5f (UPDATE system_state.last_chain_hash) and **before** `tx.commit()` (line ~697), insert:

```rust
// 5g. Create Verifactu invoice (F2) if applicable
if let Some(ref inv_svc) = self.invoice_service {
    if snapshot.status == OrderStatus::Completed {
        // Compute desglose from archived items
        let desglose = self.compute_desglose_from_items(&mut tx, order_pk).await?;
        inv_svc
            .create_order_invoice(
                &mut tx,
                order_pk,
                snapshot.subtotal + snapshot.discount - snapshot.surcharge, // pre-tax
                snapshot.tax,
                snapshot.total,
                &desglose,
            )
            .await?;
    }
}
```

Add helper method to OrderArchiveService:
```rust
async fn compute_desglose_from_items(
    &self,
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    order_pk: i64,
) -> ArchiveResult<Vec<shared::cloud::sync::TaxDesglose>> {
    let items: Vec<DesgloseItemRow> = sqlx::query_as(
        "SELECT tax_rate, SUM(line_total - tax) as base_amount, SUM(tax) as tax_amount \
         FROM archived_order_item WHERE order_pk = ? GROUP BY tax_rate",
    )
    .bind(order_pk)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| ArchiveError::Database(e.to_string()))?;

    Ok(items
        .into_iter()
        .map(|r| shared::cloud::sync::TaxDesglose {
            tax_rate: r.tax_rate,
            base_amount: r.base_amount,
            tax_amount: r.tax_amount,
        })
        .collect())
}
```

With helper struct:
```rust
#[derive(Debug, sqlx::FromRow)]
struct DesgloseItemRow {
    tax_rate: i64,
    base_amount: f64,
    tax_amount: f64,
}
```

**Step 3: Update all call sites of OrderArchiveService::new()**

Search for `OrderArchiveService::new(` across the codebase and add the `invoice_service` parameter. Initially pass `None` until Task 8 wires it up properly.

**Step 4: Verify compilation**

Run: `cargo check -p edge-server`
Expected: OK

**Step 5: Commit**

```bash
git add edge-server/src/archiving/service.rs
git commit -m "feat(verifactu): integrate F2 invoice creation into archive flow"
```

---

### Task 8: Integrate into Credit Note Flow

**Files:**
- Modify: `edge-server/src/archiving/credit_note.rs:17-33` (add InvoiceService field)
- Modify: `edge-server/src/archiving/credit_note.rs:237-265` (add invoice hook after commit)

**Step 1: Add InvoiceService to CreditNoteService**

Add `invoice_service: Option<InvoiceService>` field. Update constructor.

**Step 2: Add R5 invoice creation**

After step 9d (UPDATE system_state.last_chain_hash) and **before** `tx.commit()`:

```rust
// 9e. Create Verifactu invoice (R5) for credit note
if let Some(ref inv_svc) = self.invoice_service {
    let desglose: Vec<shared::cloud::sync::TaxDesglose> = cn_items
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut map, item| {
            let entry = map.entry(item.tax_rate).or_insert((0.0, 0.0));
            entry.0 += item.line_credit;  // base
            entry.1 += item.tax_credit;   // tax
            map
        })
        .into_iter()
        .map(|(rate, (base, tax))| shared::cloud::sync::TaxDesglose {
            tax_rate: rate,
            base_amount: base,
            tax_amount: tax,
        })
        .collect();

    inv_svc
        .create_credit_note_invoice(
            &mut tx,
            cn_pk,
            request.original_order_pk,
            subtotal_credit,
            tax_credit,
            total_credit,
            &desglose,
        )
        .await?;
}
```

**Step 3: Update all call sites of CreditNoteService::new()**

**Step 4: Verify compilation**

Run: `cargo check -p edge-server`
Expected: OK

**Step 5: Commit**

```bash
git add edge-server/src/archiving/credit_note.rs
git commit -m "feat(verifactu): integrate R5 invoice creation into credit note flow"
```

---

### Task 9: Wire Up InvoiceService in ServerState

**Files:**
- Modify: `edge-server/src/core/state.rs` (or wherever ServerState is constructed)

**Step 1: Find and modify server initialization**

Find where `OrderArchiveService::new()` and `CreditNoteService::new()` are called. Create `InvoiceService` using:
- `pool` — from ServerState
- `tz` — from Config
- `store_number` — from `ActivationData` (via `ActivationService`)
- `nif` — from `StoreInfo` (via SQLite query)
- `nombre_razon` — from `StoreInfo.name`

Pass the created `InvoiceService` to both `OrderArchiveService` and `CreditNoteService`.

If activation/store_info is not yet available (pre-activation), pass `None`.

**Step 2: Verify compilation**

Run: `cargo check -p edge-server`
Expected: OK

**Step 3: Run all tests**

Run: `cargo test --workspace --lib`
Expected: All pass (including new verifactu tests)

**Step 4: Commit**

```bash
git add edge-server/src/core/state.rs
# (and any other files touched)
git commit -m "feat(verifactu): wire InvoiceService into server initialization"
```

---

### Task 10: Final Quality Gate

**Step 1: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: Zero warnings

**Step 2: Run all tests**

Run: `cargo test --workspace --lib`
Expected: All pass

**Step 3: Run sqlx prepare (if offline mode)**

Run: `cargo sqlx prepare --workspace`

**Step 4: Final commit if needed**

```bash
git commit -m "chore: clippy fixes and sqlx prepare for verifactu"
```
