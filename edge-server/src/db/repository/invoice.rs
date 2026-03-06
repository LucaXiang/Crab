//! Invoice Repository
//!
//! CRUD operations for Verifactu invoice, invoice_desglose, and invoice_counter in SQLite.

use super::{RepoError, RepoResult};
use shared::models::invoice::{AeatStatus, Invoice, InvoiceDesglose};
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
    fecha_hora_registro: String,
    nif: String,
    nombre_razon: String,
    factura_rectificada_id: Option<i64>,
    factura_rectificada_num: Option<String>,
    factura_sustituida_id: Option<i64>,
    factura_sustituida_num: Option<String>,
    customer_nif: Option<String>,
    customer_nombre: Option<String>,
    customer_address: Option<String>,
    customer_email: Option<String>,
    customer_phone: Option<String>,
    cloud_synced: bool,
    aeat_status: String,
    created_at: i64,
}

impl InvoiceRow {
    fn into_invoice(self) -> RepoResult<Invoice> {
        Ok(Invoice {
            id: self.id,
            invoice_number: self.invoice_number,
            serie: self.serie,
            tipo_factura: self.tipo_factura.parse().map_err(|_| {
                RepoError::DataCorruption(format!("invalid tipo_factura: {}", self.tipo_factura))
            })?,
            source_type: self.source_type.parse().map_err(|_| {
                RepoError::DataCorruption(format!("invalid source_type: {}", self.source_type))
            })?,
            source_pk: self.source_pk,
            subtotal: self.subtotal,
            tax: self.tax,
            total: self.total,
            huella: self.huella,
            prev_huella: self.prev_huella,
            fecha_expedicion: self.fecha_expedicion,
            fecha_hora_registro: self.fecha_hora_registro,
            nif: self.nif,
            nombre_razon: self.nombre_razon,
            factura_rectificada_id: self.factura_rectificada_id,
            factura_rectificada_num: self.factura_rectificada_num,
            factura_sustituida_id: self.factura_sustituida_id,
            factura_sustituida_num: self.factura_sustituida_num,
            customer_nif: self.customer_nif,
            customer_nombre: self.customer_nombre,
            customer_address: self.customer_address,
            customer_email: self.customer_email,
            customer_phone: self.customer_phone,
            cloud_synced: self.cloud_synced,
            aeat_status: self.aeat_status.parse().map_err(|_| {
                RepoError::DataCorruption(format!("invalid aeat_status: {}", self.aeat_status))
            })?,
            created_at: self.created_at,
        })
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
    fecha_hora_registro, nif, nombre_razon, \
    factura_rectificada_id, factura_rectificada_num, \
    factura_sustituida_id, factura_sustituida_num, \
    customer_nif, customer_nombre, customer_address, customer_email, customer_phone, \
    cloud_synced, aeat_status, created_at";

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Read the last huella for a given serie from `invoice_counter`.
pub async fn get_last_huella(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    serie: &str,
) -> RepoResult<Option<String>> {
    let huella: Option<String> =
        sqlx::query_scalar("SELECT last_huella FROM invoice_counter WHERE serie = ?")
            .bind(serie)
            .fetch_optional(&mut **tx)
            .await?
            .flatten();
    Ok(huella)
}

/// Update the last huella for a given serie in `invoice_counter`.
pub async fn update_last_huella(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    serie: &str,
    huella: &str,
) -> RepoResult<()> {
    sqlx::query(
        "INSERT INTO invoice_counter (serie, date_str, last_number, last_huella) \
         VALUES (?1, '', 0, ?2) \
         ON CONFLICT(serie) DO UPDATE SET last_huella = ?2",
    )
    .bind(serie)
    .bind(huella)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

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
          fecha_hora_registro, nif, nombre_razon, \
          factura_rectificada_id, factura_rectificada_num, \
          factura_sustituida_id, factura_sustituida_num, \
          customer_nif, customer_nombre, customer_address, customer_email, customer_phone, \
          cloud_synced, aeat_status, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27)",
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
    .bind(&invoice.fecha_hora_registro)
    .bind(&invoice.nif)
    .bind(&invoice.nombre_razon)
    .bind(invoice.factura_rectificada_id)
    .bind(&invoice.factura_rectificada_num)
    .bind(invoice.factura_sustituida_id)
    .bind(&invoice.factura_sustituida_num)
    .bind(&invoice.customer_nif)
    .bind(&invoice.customer_nombre)
    .bind(&invoice.customer_address)
    .bind(&invoice.customer_email)
    .bind(&invoice.customer_phone)
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
    tax_rate: i32,
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

    rows.into_iter().map(InvoiceRow::into_invoice).collect()
}

/// Get tax breakdown lines for an invoice (within a transaction).
pub async fn get_desglose_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    invoice_id: i64,
) -> RepoResult<Vec<InvoiceDesglose>> {
    let rows = sqlx::query_as::<_, InvoiceDesglose>(
        "SELECT id, invoice_id, tax_rate, base_amount, tax_amount \
         FROM invoice_desglose WHERE invoice_id = ? ORDER BY tax_rate",
    )
    .bind(invoice_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows)
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

    row.map(InvoiceRow::into_invoice).transpose()
}

/// Get an invoice by ID.
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Invoice>> {
    let row = sqlx::query_as::<_, InvoiceRow>(&format!(
        "SELECT {INVOICE_COLUMNS} FROM invoice WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;

    row.map(InvoiceRow::into_invoice).transpose()
}

/// List invoice IDs not yet synced to cloud (ordered by id for chain consistency).
pub async fn list_unsynced_ids(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<i64>> {
    let rows = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM invoice WHERE cloud_synced = 0 ORDER BY id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Build InvoiceSync payload for cloud sync.
pub async fn build_sync(
    pool: &SqlitePool,
    invoice_id: i64,
) -> RepoResult<shared::cloud::sync::InvoiceSync> {
    use rust_decimal::Decimal;
    use shared::cloud::sync::{InvoiceSync, TaxDesglose};

    let invoice = get_by_id(pool, invoice_id)
        .await?
        .ok_or_else(|| super::RepoError::NotFound(format!("invoice {invoice_id}")))?;

    let desglose_rows = get_desglose(pool, invoice_id).await?;
    let desglose: Vec<TaxDesglose> = desglose_rows
        .into_iter()
        .map(|d| {
            let base_amount = Decimal::try_from(d.base_amount).map_err(|e| {
                super::RepoError::Database(format!(
                    "desglose base_amount f64→Decimal: {e} (value={})",
                    d.base_amount
                ))
            })?;
            let tax_amount = Decimal::try_from(d.tax_amount).map_err(|e| {
                super::RepoError::Database(format!(
                    "desglose tax_amount f64→Decimal: {e} (value={})",
                    d.tax_amount
                ))
            })?;
            Ok(TaxDesglose {
                tax_rate: d.tax_rate,
                base_amount,
                tax_amount,
            })
        })
        .collect::<RepoResult<Vec<_>>>()?;

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
        fecha_hora_registro: invoice.fecha_hora_registro,
        nif: invoice.nif,
        nombre_razon: invoice.nombre_razon,
        factura_rectificada_id: invoice.factura_rectificada_id,
        factura_rectificada_num: invoice.factura_rectificada_num,
        factura_sustituida_id: invoice.factura_sustituida_id,
        factura_sustituida_num: invoice.factura_sustituida_num,
        customer_nif: invoice.customer_nif,
        customer_nombre: invoice.customer_nombre,
        customer_address: invoice.customer_address,
        customer_email: invoice.customer_email,
        customer_phone: invoice.customer_phone,
        created_at: invoice.created_at,
    })
}

/// Restore invoice counter from cloud recovery state.
///
/// Parses "SERIE-YYYYMMDD-NNNN" format and upserts the invoice_counter row.
pub async fn restore_invoice_counter(
    pool: &SqlitePool,
    last_invoice_number: &str,
    last_huella: Option<&str>,
) -> RepoResult<()> {
    // Parse "SERIE-YYYYMMDD-NNNN" format (rsplit to handle serie containing dashes)
    let parts: Vec<&str> = last_invoice_number.rsplitn(3, '-').collect();
    if parts.len() != 3 {
        return Err(RepoError::Database(format!(
            "Invalid invoice number format: {last_invoice_number}"
        )));
    }
    let number: i64 = parts[0]
        .parse()
        .map_err(|_| RepoError::Database(format!("Invalid invoice counter: {}", parts[0])))?;
    let date_str = parts[1];
    let serie = parts[2];

    sqlx::query(
        "INSERT INTO invoice_counter (serie, date_str, last_number, last_huella) \
         VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(serie) DO UPDATE SET date_str = ?2, last_number = ?3, last_huella = ?4",
    )
    .bind(serie)
    .bind(date_str)
    .bind(number)
    .bind(last_huella)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update the AEAT status of an invoice (cloud→edge callback).
/// Cloud is authoritative for aeat_status; edge stores only the status string.
pub async fn update_aeat_status(
    pool: &SqlitePool,
    invoice_number: &str,
    aeat_status: AeatStatus,
) -> RepoResult<bool> {
    let result = sqlx::query("UPDATE invoice SET aeat_status = ?1 WHERE invoice_number = ?2")
        .bind(aeat_status.as_str())
        .bind(invoice_number)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ---------------------------------------------------------------------------
// Invoice list (paginated, with filters)
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct InvoiceListRow {
    pub id: i64,
    pub invoice_number: String,
    pub tipo_factura: String,
    pub source_type: String,
    pub source_pk: i64,
    pub total: f64,
    pub tax: f64,
    pub aeat_status: String,
    pub created_at: i64,
}

/// List invoices with optional filters, paginated. Returns (rows, total_count).
pub async fn list_paginated(
    pool: &SqlitePool,
    from: i64,
    to: i64,
    tipo: Option<&str>,
    aeat_status: Option<&str>,
    limit: i32,
    offset: i32,
) -> RepoResult<(Vec<InvoiceListRow>, i64)> {
    let mut where_clauses = vec!["created_at >= ?1 AND created_at < ?2".to_string()];
    let mut bind_idx = 3;

    if tipo.is_some() {
        where_clauses.push(format!("tipo_factura = ?{bind_idx}"));
        bind_idx += 1;
    }
    if aeat_status.is_some() {
        where_clauses.push(format!("aeat_status = ?{bind_idx}"));
        bind_idx += 1;
    }

    let where_clause = where_clauses.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM invoice WHERE {where_clause}");
    let list_sql = format!(
        "SELECT id, invoice_number, tipo_factura, source_type, source_pk, \
         total, tax, aeat_status, created_at \
         FROM invoice WHERE {where_clause} ORDER BY id DESC LIMIT ?{bind_idx} OFFSET ?{}",
        bind_idx + 1
    );

    // Build count query
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql).bind(from).bind(to);
    if let Some(t) = tipo {
        count_q = count_q.bind(t);
    }
    if let Some(s) = aeat_status {
        count_q = count_q.bind(s);
    }
    let total = count_q.fetch_one(pool).await?;

    // Build list query
    let mut list_q = sqlx::query_as::<_, InvoiceListRow>(&list_sql)
        .bind(from)
        .bind(to);
    if let Some(t) = tipo {
        list_q = list_q.bind(t);
    }
    if let Some(s) = aeat_status {
        list_q = list_q.bind(s);
    }
    list_q = list_q.bind(limit).bind(offset);
    let rows = list_q.fetch_all(pool).await?;

    Ok((rows, total))
}

/// Find all invoices linked to a given order (F2 for the order + R5 for its credit notes).
pub async fn find_by_order(pool: &SqlitePool, order_pk: i64) -> RepoResult<Vec<Invoice>> {
    // Get direct F2 invoice + any R5 invoices whose source is a credit_note of this order
    let rows = sqlx::query_as::<_, InvoiceRow>(&format!(
        "SELECT {INVOICE_COLUMNS} FROM invoice \
         WHERE (source_type = 'ORDER' AND source_pk = ?1) \
            OR (source_type = 'CREDIT_NOTE' AND source_pk IN \
                (SELECT id FROM credit_note WHERE original_order_pk = ?1)) \
         ORDER BY id"
    ))
    .bind(order_pk)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(InvoiceRow::into_invoice).collect()
}
