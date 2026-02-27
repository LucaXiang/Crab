//! Invoice Repository
//!
//! CRUD operations for Verifactu invoice, invoice_desglose, and invoice_counter in SQLite.

use super::RepoResult;
use shared::models::invoice::{
    AeatStatus, Invoice, InvoiceDesglose, InvoiceSourceType, TipoFactura,
};
use shared::util::snowflake_id;
use sqlx::SqlitePool;

// ---------------------------------------------------------------------------
// Internal row type — sqlx cannot auto-map String → enum, so we read into
// plain String fields and convert manually.
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct InvoiceRow {
    id: i64,
    invoice_number: String,
    serie: String,
    tipo_factura: String,
    source_type: String,
    source_pk: i64,
    subtotal: f64,
    tax: f64,
    total: f64,
    huella: String,
    prev_huella: Option<String>,
    fecha_expedicion: String,
    nif: String,
    nombre_razon: String,
    factura_rectificada_id: Option<i64>,
    factura_rectificada_num: Option<String>,
    cloud_synced: bool,
    aeat_status: String,
    created_at: i64,
}

impl InvoiceRow {
    fn into_invoice(self) -> Invoice {
        Invoice {
            id: self.id,
            invoice_number: self.invoice_number,
            serie: self.serie,
            tipo_factura: self.tipo_factura.parse().unwrap_or(TipoFactura::F2),
            source_type: self.source_type.parse().unwrap_or(InvoiceSourceType::Order),
            source_pk: self.source_pk,
            subtotal: self.subtotal,
            tax: self.tax,
            total: self.total,
            huella: self.huella,
            prev_huella: self.prev_huella,
            fecha_expedicion: self.fecha_expedicion,
            nif: self.nif,
            nombre_razon: self.nombre_razon,
            factura_rectificada_id: self.factura_rectificada_id,
            factura_rectificada_num: self.factura_rectificada_num,
            cloud_synced: self.cloud_synced,
            aeat_status: self.aeat_status.parse().unwrap_or(AeatStatus::Pending),
            created_at: self.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct CounterRow {
    date_str: String,
    last_number: i64,
}

// ---------------------------------------------------------------------------
// Constants for SELECT column list (avoids repetition)
// ---------------------------------------------------------------------------

const INVOICE_COLUMNS: &str = "\
    id, invoice_number, serie, tipo_factura, source_type, source_pk, \
    subtotal, tax, total, huella, prev_huella, fecha_expedicion, \
    nif, nombre_razon, factura_rectificada_id, factura_rectificada_num, \
    cloud_synced, aeat_status, created_at";

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Insert a new invoice. Returns the generated snowflake id.
pub async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    invoice: &Invoice,
) -> RepoResult<i64> {
    let id = snowflake_id();

    sqlx::query(
        "INSERT INTO invoice \
         (id, invoice_number, serie, tipo_factura, source_type, source_pk, \
          subtotal, tax, total, huella, prev_huella, fecha_expedicion, \
          nif, nombre_razon, factura_rectificada_id, factura_rectificada_num, \
          cloud_synced, aeat_status, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
    )
    .bind(id)
    .bind(&invoice.invoice_number)
    .bind(&invoice.serie)
    .bind(invoice.tipo_factura.as_str())
    .bind(invoice.source_type.as_str())
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
    .bind(invoice.cloud_synced)
    .bind(invoice.aeat_status.as_str())
    .bind(invoice.created_at)
    .execute(&mut **tx)
    .await?;

    Ok(id)
}

/// Insert a tax breakdown line (desglose) for an invoice.
pub async fn insert_desglose(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    invoice_id: i64,
    tax_rate: i64,
    base_amount: f64,
    tax_amount: f64,
) -> RepoResult<()> {
    let id = snowflake_id();

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

/// Generate the next invoice number for a given serie and date.
///
/// Uses the `invoice_counter` table for crash-safe sequential numbering.
/// If the date has changed (or no counter row exists), we double-check
/// against actual invoices to avoid gaps or duplicates after a crash.
///
/// Returns formatted: `"{serie}-{date_str}-{0001}"`.
pub async fn next_invoice_number(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    serie: &str,
    date_str: &str,
) -> RepoResult<String> {
    let counter = sqlx::query_as::<_, CounterRow>(
        "SELECT date_str, last_number FROM invoice_counter WHERE serie = ?",
    )
    .bind(serie)
    .fetch_optional(&mut **tx)
    .await?;

    let next = match counter {
        Some(row) if row.date_str == date_str => {
            // Same date — just increment
            row.last_number + 1
        }
        _ => {
            // Different date or no row — check actual invoices for safety
            let prefix = format!("{serie}-{date_str}-");
            let max_num: Option<String> = sqlx::query_scalar(
                "SELECT MAX(invoice_number) FROM invoice \
                 WHERE serie = ? AND invoice_number LIKE ?",
            )
            .bind(serie)
            .bind(format!("{prefix}%"))
            .fetch_one(&mut **tx)
            .await?;

            match max_num {
                Some(num) => {
                    // Extract trailing number: "SERIE-20260227-0003" → 3
                    let suffix = num.strip_prefix(&prefix).unwrap_or("0");
                    suffix.parse::<i64>().unwrap_or(0) + 1
                }
                None => 1,
            }
        }
    };

    // UPSERT the counter
    sqlx::query(
        "INSERT INTO invoice_counter (serie, date_str, last_number) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(serie) DO UPDATE SET date_str = ?2, last_number = ?3",
    )
    .bind(serie)
    .bind(date_str)
    .bind(next)
    .execute(&mut **tx)
    .await?;

    Ok(format!("{serie}-{date_str}-{next:04}"))
}

/// List invoices not yet synced to cloud.
pub async fn list_unsynced(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<Invoice>> {
    let rows = sqlx::query_as::<_, InvoiceRow>(&format!(
        "SELECT {INVOICE_COLUMNS} FROM invoice WHERE cloud_synced = 0 ORDER BY id LIMIT ?"
    ))
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(InvoiceRow::into_invoice).collect())
}

/// Get tax breakdown lines for an invoice.
pub async fn get_desglose(pool: &SqlitePool, invoice_id: i64) -> RepoResult<Vec<InvoiceDesglose>> {
    let rows = sqlx::query_as::<_, InvoiceDesglose>(
        "SELECT id, invoice_id, tax_rate, base_amount, tax_amount \
         FROM invoice_desglose WHERE invoice_id = ? ORDER BY tax_rate",
    )
    .bind(invoice_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Mark invoices as synced to cloud.
pub async fn mark_synced(pool: &SqlitePool, ids: &[i64]) -> RepoResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE invoice SET cloud_synced = 1 WHERE id IN ({placeholders})");
    let mut query = sqlx::query(&sql);
    for id in ids {
        query = query.bind(id);
    }
    query.execute(pool).await?;
    Ok(())
}

/// Find the F2 (original) invoice for a given order.
pub async fn find_order_invoice(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    order_pk: i64,
) -> RepoResult<Option<Invoice>> {
    let row = sqlx::query_as::<_, InvoiceRow>(&format!(
        "SELECT {INVOICE_COLUMNS} FROM invoice \
         WHERE source_type = 'ORDER' AND source_pk = ? AND tipo_factura = 'F2'"
    ))
    .bind(order_pk)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(InvoiceRow::into_invoice))
}
