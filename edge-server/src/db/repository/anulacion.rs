//! Invoice Anulación Repository
//!
//! CRUD operations for Verifactu invoice anulación (RegistroFacturaBaja) in SQLite.

use super::RepoResult;
use shared::cloud::sync::AnulacionSync;
use shared::models::invoice::{AeatStatus, AnulacionReason, InvoiceAnulacion};
use shared::util::snowflake_id;
use sqlx::SqlitePool;

// ---------------------------------------------------------------------------
// Internal row type
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct AnulacionRow {
    id: i64,
    anulacion_number: String,
    serie: String,
    original_invoice_id: i64,
    original_invoice_number: String,
    huella: String,
    prev_huella: Option<String>,
    fecha_expedicion: String,
    fecha_hora_registro: String,
    nif: String,
    nombre_razon: String,
    original_order_pk: i64,
    reason: String,
    note: Option<String>,
    operator_id: i64,
    operator_name: String,
    cloud_synced: bool,
    aeat_status: String,
    created_at: i64,
}

impl AnulacionRow {
    fn into_anulacion(self) -> InvoiceAnulacion {
        InvoiceAnulacion {
            id: self.id,
            anulacion_number: self.anulacion_number,
            serie: self.serie,
            original_invoice_id: self.original_invoice_id,
            original_invoice_number: self.original_invoice_number,
            huella: self.huella,
            prev_huella: self.prev_huella,
            fecha_expedicion: self.fecha_expedicion,
            fecha_hora_registro: self.fecha_hora_registro,
            nif: self.nif,
            nombre_razon: self.nombre_razon,
            original_order_pk: self.original_order_pk,
            reason: self.reason.parse().unwrap_or(AnulacionReason::Other),
            note: self.note,
            operator_id: self.operator_id,
            operator_name: self.operator_name,
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

const ANULACION_COLUMNS: &str = "\
    id, anulacion_number, serie, original_invoice_id, original_invoice_number, \
    huella, prev_huella, fecha_expedicion, fecha_hora_registro, nif, nombre_razon, \
    original_order_pk, reason, note, operator_id, operator_name, \
    cloud_synced, aeat_status, created_at";

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Insert a new invoice anulación. Returns the generated snowflake id.
pub async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    anulacion: &InvoiceAnulacion,
) -> RepoResult<i64> {
    let id = snowflake_id();

    sqlx::query(
        "INSERT INTO invoice_anulacion \
         (id, anulacion_number, serie, original_invoice_id, original_invoice_number, \
          huella, prev_huella, fecha_expedicion, fecha_hora_registro, nif, nombre_razon, \
          original_order_pk, reason, note, operator_id, operator_name, \
          cloud_synced, aeat_status, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",
    )
    .bind(id)
    .bind(&anulacion.anulacion_number)
    .bind(&anulacion.serie)
    .bind(anulacion.original_invoice_id)
    .bind(&anulacion.original_invoice_number)
    .bind(&anulacion.huella)
    .bind(&anulacion.prev_huella)
    .bind(&anulacion.fecha_expedicion)
    .bind(&anulacion.fecha_hora_registro)
    .bind(&anulacion.nif)
    .bind(&anulacion.nombre_razon)
    .bind(anulacion.original_order_pk)
    .bind(anulacion.reason.as_str())
    .bind(&anulacion.note)
    .bind(anulacion.operator_id)
    .bind(&anulacion.operator_name)
    .bind(anulacion.cloud_synced)
    .bind(anulacion.aeat_status.as_str())
    .bind(anulacion.created_at)
    .execute(&mut **tx)
    .await?;

    Ok(id)
}

/// Generate the next anulación number for a given serie and date.
///
/// Format: `"AN-{serie}-{date_str}-{0001}"`
pub async fn next_anulacion_number(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    serie: &str,
    date_str: &str,
) -> RepoResult<String> {
    let counter = sqlx::query_as::<_, CounterRow>(
        "SELECT date_str, last_number FROM anulacion_counter WHERE serie = ?",
    )
    .bind(serie)
    .fetch_optional(&mut **tx)
    .await?;

    let next = match counter {
        Some(row) if row.date_str == date_str => row.last_number + 1,
        _ => {
            let prefix = format!("AN-{serie}-{date_str}-");
            let max_num: Option<String> = sqlx::query_scalar(
                "SELECT MAX(anulacion_number) FROM invoice_anulacion \
                 WHERE serie = ? AND anulacion_number LIKE ?",
            )
            .bind(serie)
            .bind(format!("{prefix}%"))
            .fetch_one(&mut **tx)
            .await?;

            match max_num {
                Some(num) => {
                    let suffix = num.strip_prefix(&prefix).unwrap_or("0");
                    suffix.parse::<i64>().unwrap_or(0) + 1
                }
                None => 1,
            }
        }
    };

    sqlx::query(
        "INSERT INTO anulacion_counter (serie, date_str, last_number) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(serie) DO UPDATE SET date_str = ?2, last_number = ?3",
    )
    .bind(serie)
    .bind(date_str)
    .bind(next)
    .execute(&mut **tx)
    .await?;

    Ok(format!("AN-{serie}-{date_str}-{next:04}"))
}

/// Get an anulación by ID.
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<InvoiceAnulacion>> {
    let row = sqlx::query_as::<_, AnulacionRow>(&format!(
        "SELECT {ANULACION_COLUMNS} FROM invoice_anulacion WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(AnulacionRow::into_anulacion))
}

/// Check if an order already has an anulación.
pub async fn has_anulacion(pool: &SqlitePool, order_pk: i64) -> RepoResult<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM invoice_anulacion WHERE original_order_pk = ?")
            .bind(order_pk)
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

/// List anulación IDs not yet synced to cloud (ordered by id for chain consistency).
pub async fn list_unsynced_ids(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<i64>> {
    let rows = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM invoice_anulacion WHERE cloud_synced = 0 ORDER BY id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Mark anulaciones as synced to cloud.
pub async fn mark_synced(pool: &SqlitePool, ids: &[i64]) -> RepoResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE invoice_anulacion SET cloud_synced = 1 WHERE id IN ({placeholders})");
    let mut query = sqlx::query(&sql);
    for id in ids {
        query = query.bind(id);
    }
    query.execute(pool).await?;
    Ok(())
}

/// Build AnulacionSync payload for cloud sync.
pub async fn build_sync(pool: &SqlitePool, anulacion_id: i64) -> RepoResult<AnulacionSync> {
    let anulacion = get_by_id(pool, anulacion_id)
        .await?
        .ok_or_else(|| super::RepoError::NotFound(format!("anulacion {anulacion_id}")))?;

    // Get chain_entry hash data
    let (prev_hash, curr_hash) = sqlx::query_as::<_, (String, String)>(
        "SELECT prev_hash, curr_hash FROM chain_entry \
         WHERE entry_type = 'ANULACION' AND entry_pk = ?",
    )
    .bind(anulacion_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));

    Ok(AnulacionSync {
        id: anulacion.id,
        anulacion_number: anulacion.anulacion_number,
        serie: anulacion.serie,
        original_invoice_id: anulacion.original_invoice_id,
        original_invoice_number: anulacion.original_invoice_number,
        huella: anulacion.huella,
        prev_huella: anulacion.prev_huella,
        fecha_expedicion: anulacion.fecha_expedicion,
        fecha_hora_registro: anulacion.fecha_hora_registro,
        nif: anulacion.nif,
        nombre_razon: anulacion.nombre_razon,
        original_order_pk: anulacion.original_order_pk,
        reason: anulacion.reason.as_str().to_string(),
        note: anulacion.note,
        operator_id: anulacion.operator_id,
        operator_name: anulacion.operator_name,
        prev_hash,
        curr_hash,
        created_at: anulacion.created_at,
    })
}

/// Update the AEAT status of an anulación (cloud→edge callback).
pub async fn update_aeat_status(
    pool: &SqlitePool,
    anulacion_number: &str,
    aeat_status: AeatStatus,
) -> RepoResult<bool> {
    let result =
        sqlx::query("UPDATE invoice_anulacion SET aeat_status = ?1 WHERE anulacion_number = ?2")
            .bind(aeat_status.as_str())
            .bind(anulacion_number)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

/// Find anulacion for a given order.
pub async fn find_by_order(
    pool: &SqlitePool,
    order_pk: i64,
) -> RepoResult<Option<InvoiceAnulacion>> {
    let row = sqlx::query_as::<_, AnulacionRow>(&format!(
        "SELECT {ANULACION_COLUMNS} FROM invoice_anulacion WHERE original_order_pk = ?"
    ))
    .bind(order_pk)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(AnulacionRow::into_anulacion))
}
