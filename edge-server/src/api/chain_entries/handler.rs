//! Chain Entries API Handler
//!
//! GET /api/chain-entries              — ORDER + CREDIT_NOTE 混合时间线
//! GET /api/chain-entries/credit-note/:id — 退款凭证详情（含 CreditNoteType + hash）

use crate::core::ServerState;
use crate::utils::{AppError, AppResult};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
// ── 查询参数 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChainEntryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

// ── 列表响应类型 ──────────────────────────────────────────────────────────────

/// chain_entry 列表条目 — ORDER 和 CREDIT_NOTE 统一返回
#[derive(Debug, Serialize)]
pub struct ChainEntryItem {
    pub chain_id: i64,
    pub entry_type: String,
    pub entry_pk: i64,
    pub display_number: String,
    pub status: Option<String>,
    pub amount: f64,
    pub created_at: i64,
    pub prev_hash: String,
    pub curr_hash: String,
    pub original_order_pk: Option<i64>,
    pub original_receipt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChainEntryListResponse {
    pub entries: Vec<ChainEntryItem>,
    pub total: i64,
}

// ── 退款凭证详情 ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreditNoteDetailResponse {
    pub id: i64,
    pub credit_note_number: String,
    pub original_order_pk: i64,
    pub original_receipt: String,
    pub subtotal_credit: f64,
    pub tax_credit: f64,
    pub total_credit: f64,
    pub refund_method: String,
    pub reason: String,
    pub note: Option<String>,
    pub operator_id: i64,
    pub operator_name: String,
    pub authorizer_id: Option<i64>,
    pub authorizer_name: Option<String>,
    pub created_at: i64,
    pub prev_hash: String,
    pub curr_hash: String,
    pub items: Vec<CreditNoteItemRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CreditNoteItemRow {
    pub id: i64,
    pub original_instance_id: String,
    pub item_name: String,
    pub quantity: i64,
    pub unit_price: f64,
    pub line_credit: f64,
    pub tax_rate: i64,
    pub tax_credit: f64,
}

// ── Handler: list ────────────────────────────────────────────────────────────

/// SQL: UNION ALL 查询 ORDER + CREDIT_NOTE + ANULACION + UPGRADE，按 chain_id DESC 排序
///
/// 搜索参数需要绑定多次（SQLite 不支持参数复用），
/// ORDER 分支绑定一次、CREDIT_NOTE 分支绑定一次、ANULACION 分支绑定一次、UPGRADE 分支绑定一次。
const LIST_SQL: &str = "\
SELECT
    ce.id          AS chain_id,
    ce.entry_type,
    ce.entry_pk,
    COALESCE(ao.receipt_number, CAST(ao.order_id AS TEXT)) AS display_number,
    UPPER(ao.status) AS status,
    ao.total_amount AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    NULL           AS original_order_pk,
    NULL           AS original_receipt
FROM chain_entry ce
JOIN archived_order ao ON ao.id = ce.entry_pk
WHERE ce.entry_type = 'ORDER'
  AND (? IS NULL OR LOWER(COALESCE(ao.receipt_number, CAST(ao.order_id AS TEXT))) LIKE ?)

UNION ALL

SELECT
    ce.id          AS chain_id,
    ce.entry_type,
    ce.entry_pk,
    cn.credit_note_number AS display_number,
    NULL           AS status,
    cn.total_credit AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    cn.original_order_pk,
    cn.original_receipt
FROM chain_entry ce
JOIN credit_note cn ON cn.id = ce.entry_pk
WHERE ce.entry_type = 'CREDIT_NOTE'
  AND (? IS NULL OR LOWER(cn.credit_note_number) LIKE ?
       OR LOWER(cn.original_receipt) LIKE ?)

UNION ALL

SELECT
    ce.id          AS chain_id,
    ce.entry_type,
    ce.entry_pk,
    ia.anulacion_number AS display_number,
    'ANULADA'      AS status,
    0.0            AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    ia.original_order_pk,
    ia.original_invoice_number AS original_receipt
FROM chain_entry ce
JOIN invoice_anulacion ia ON ia.id = ce.entry_pk
WHERE ce.entry_type = 'ANULACION'
  AND (? IS NULL OR LOWER(ia.anulacion_number) LIKE ?
       OR LOWER(ia.original_invoice_number) LIKE ?)

UNION ALL

SELECT
    ce.id          AS chain_id,
    ce.entry_type,
    ce.entry_pk,
    inv.invoice_number AS display_number,
    'F3'           AS status,
    inv.total      AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    inv.source_pk  AS original_order_pk,
    inv.factura_sustituida_num AS original_receipt
FROM chain_entry ce
JOIN invoice inv ON inv.id = ce.entry_pk
WHERE ce.entry_type = 'UPGRADE'
  AND (? IS NULL OR LOWER(inv.invoice_number) LIKE ?
       OR LOWER(inv.factura_sustituida_num) LIKE ?)

ORDER BY chain_id DESC
LIMIT ? OFFSET ?";

const COUNT_SQL: &str = "\
SELECT COUNT(*) FROM (
    SELECT ce.id FROM chain_entry ce
    JOIN archived_order ao ON ao.id = ce.entry_pk
    WHERE ce.entry_type = 'ORDER'
      AND (? IS NULL OR LOWER(COALESCE(ao.receipt_number, CAST(ao.order_id AS TEXT))) LIKE ?)
    UNION ALL
    SELECT ce.id FROM chain_entry ce
    JOIN credit_note cn ON cn.id = ce.entry_pk
    WHERE ce.entry_type = 'CREDIT_NOTE'
      AND (? IS NULL OR LOWER(cn.credit_note_number) LIKE ?
           OR LOWER(cn.original_receipt) LIKE ?)
    UNION ALL
    SELECT ce.id FROM chain_entry ce
    JOIN invoice_anulacion ia ON ia.id = ce.entry_pk
    WHERE ce.entry_type = 'ANULACION'
      AND (? IS NULL OR LOWER(ia.anulacion_number) LIKE ?
           OR LOWER(ia.original_invoice_number) LIKE ?)
    UNION ALL
    SELECT ce.id FROM chain_entry ce
    JOIN invoice inv ON inv.id = ce.entry_pk
    WHERE ce.entry_type = 'UPGRADE'
      AND (? IS NULL OR LOWER(inv.invoice_number) LIKE ?
           OR LOWER(inv.factura_sustituida_num) LIKE ?)
)";

/// 内部 row 映射（含 original_total 用于派生 CreditNoteType）
#[derive(Debug, sqlx::FromRow)]
struct ChainEntryRaw {
    chain_id: i64,
    entry_type: String,
    entry_pk: i64,
    display_number: String,
    status: Option<String>,
    amount: f64,
    created_at: i64,
    prev_hash: String,
    curr_hash: String,
    original_order_pk: Option<i64>,
    original_receipt: Option<String>,
}

pub async fn list(
    State(state): State<ServerState>,
    Query(params): Query<ChainEntryQuery>,
) -> AppResult<Json<ChainEntryListResponse>> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);
    let search_pattern = params
        .search
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    // COUNT — 11 binds: (2) ORDER + (3) CREDIT_NOTE + (3) ANULACION + (3) UPGRADE
    let total: i64 = sqlx::query_scalar(COUNT_SQL)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // LIST — 13 binds: (2) ORDER + (3) CREDIT_NOTE + (3) ANULACION + (3) UPGRADE + limit + offset
    let rows = sqlx::query_as::<_, ChainEntryRaw>(LIST_SQL)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let entries = rows
        .into_iter()
        .map(|r| ChainEntryItem {
            chain_id: r.chain_id,
            entry_type: r.entry_type,
            entry_pk: r.entry_pk,
            display_number: r.display_number,
            status: r.status,
            amount: r.amount,
            created_at: r.created_at,
            prev_hash: r.prev_hash,
            curr_hash: r.curr_hash,
            original_order_pk: r.original_order_pk,
            original_receipt: r.original_receipt,
        })
        .collect();

    Ok(Json(ChainEntryListResponse { entries, total }))
}

// ── Handler: get_credit_note_detail ──────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct CnDetailRow {
    id: i64,
    credit_note_number: String,
    original_order_pk: i64,
    original_receipt: String,
    subtotal_credit: f64,
    tax_credit: f64,
    total_credit: f64,
    refund_method: String,
    reason: String,
    note: Option<String>,
    operator_id: i64,
    operator_name: String,
    authorizer_id: Option<i64>,
    authorizer_name: Option<String>,
    created_at: i64,
    prev_hash: String,
    curr_hash: String,
}

pub async fn get_credit_note_detail(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<CreditNoteDetailResponse>> {
    let cn = sqlx::query_as::<_, CnDetailRow>(
        "SELECT cn.id, cn.credit_note_number, cn.original_order_pk, cn.original_receipt, \
         cn.subtotal_credit, cn.tax_credit, cn.total_credit, \
         cn.refund_method, cn.reason, cn.note, \
         cn.operator_id, cn.operator_name, cn.authorizer_id, cn.authorizer_name, \
         cn.created_at, \
         ce.prev_hash, ce.curr_hash \
         FROM credit_note cn \
         JOIN chain_entry ce ON ce.entry_type = 'CREDIT_NOTE' AND ce.entry_pk = cn.id \
         WHERE cn.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .ok_or_else(|| AppError::not_found(format!("Credit note {id} not found")))?;

    let items = sqlx::query_as::<_, CreditNoteItemRow>(
        "SELECT id, original_instance_id, item_name, quantity, unit_price, \
         line_credit, tax_rate, tax_credit \
         FROM credit_note_item WHERE credit_note_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(CreditNoteDetailResponse {
        id: cn.id,
        credit_note_number: cn.credit_note_number,
        original_order_pk: cn.original_order_pk,
        original_receipt: cn.original_receipt,
        subtotal_credit: cn.subtotal_credit,
        tax_credit: cn.tax_credit,
        total_credit: cn.total_credit,
        refund_method: cn.refund_method,
        reason: cn.reason,
        note: cn.note,
        operator_id: cn.operator_id,
        operator_name: cn.operator_name,
        authorizer_id: cn.authorizer_id,
        authorizer_name: cn.authorizer_name,
        created_at: cn.created_at,
        prev_hash: cn.prev_hash,
        curr_hash: cn.curr_hash,
        items,
    }))
}

// ── Handler: get_anulacion_detail ────────────────────────────────────────────

/// Anulación detail response for the chain timeline
#[derive(Debug, Serialize)]
pub struct AnulacionDetailResponse {
    pub id: i64,
    pub anulacion_number: String,
    pub serie: String,
    pub original_invoice_id: i64,
    pub original_invoice_number: String,
    pub original_order_pk: i64,
    pub reason: String,
    pub note: Option<String>,
    pub operator_id: i64,
    pub operator_name: String,
    pub huella: String,
    pub aeat_status: String,
    pub created_at: i64,
    pub prev_hash: String,
    pub curr_hash: String,
}

#[derive(Debug, sqlx::FromRow)]
struct AnulacionDetailRow {
    id: i64,
    anulacion_number: String,
    serie: String,
    original_invoice_id: i64,
    original_invoice_number: String,
    original_order_pk: i64,
    reason: String,
    note: Option<String>,
    operator_id: i64,
    operator_name: String,
    huella: String,
    aeat_status: String,
    created_at: i64,
    prev_hash: String,
    curr_hash: String,
}

pub async fn get_anulacion_detail(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<AnulacionDetailResponse>> {
    let row = sqlx::query_as::<_, AnulacionDetailRow>(
        "SELECT ia.id, ia.anulacion_number, ia.serie, \
         ia.original_invoice_id, ia.original_invoice_number, ia.original_order_pk, \
         ia.reason, ia.note, ia.operator_id, ia.operator_name, \
         ia.huella, ia.aeat_status, ia.created_at, \
         ce.prev_hash, ce.curr_hash \
         FROM invoice_anulacion ia \
         JOIN chain_entry ce ON ce.entry_type = 'ANULACION' AND ce.entry_pk = ia.id \
         WHERE ia.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .ok_or_else(|| AppError::not_found(format!("Anulación {id} not found")))?;

    Ok(Json(AnulacionDetailResponse {
        id: row.id,
        anulacion_number: row.anulacion_number,
        serie: row.serie,
        original_invoice_id: row.original_invoice_id,
        original_invoice_number: row.original_invoice_number,
        original_order_pk: row.original_order_pk,
        reason: row.reason,
        note: row.note,
        operator_id: row.operator_id,
        operator_name: row.operator_name,
        huella: row.huella,
        aeat_status: row.aeat_status,
        created_at: row.created_at,
        prev_hash: row.prev_hash,
        curr_hash: row.curr_hash,
    }))
}

// ── Handler: get_upgrade_detail ──────────────────────────────────────────────

/// Upgrade (F3) detail response for the chain timeline
#[derive(Debug, Serialize)]
pub struct UpgradeDetailResponse {
    pub id: i64,
    pub invoice_number: String,
    pub serie: String,
    pub tipo_factura: String,
    pub source_pk: i64,
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,
    pub factura_sustituida_id: Option<i64>,
    pub factura_sustituida_num: Option<String>,
    pub customer_nif: Option<String>,
    pub customer_nombre: Option<String>,
    pub customer_address: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub huella: String,
    pub aeat_status: String,
    pub created_at: i64,
    pub prev_hash: String,
    pub curr_hash: String,
}

#[derive(Debug, sqlx::FromRow)]
struct UpgradeDetailRow {
    id: i64,
    invoice_number: String,
    serie: String,
    tipo_factura: String,
    source_pk: i64,
    subtotal: f64,
    tax: f64,
    total: f64,
    factura_sustituida_id: Option<i64>,
    factura_sustituida_num: Option<String>,
    customer_nif: Option<String>,
    customer_nombre: Option<String>,
    customer_address: Option<String>,
    customer_email: Option<String>,
    customer_phone: Option<String>,
    huella: String,
    aeat_status: String,
    created_at: i64,
    prev_hash: String,
    curr_hash: String,
}

pub async fn get_upgrade_detail(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<UpgradeDetailResponse>> {
    let row = sqlx::query_as::<_, UpgradeDetailRow>(
        "SELECT inv.id, inv.invoice_number, inv.serie, inv.tipo_factura, inv.source_pk, \
         inv.subtotal, inv.tax, inv.total, \
         inv.factura_sustituida_id, inv.factura_sustituida_num, \
         inv.customer_nif, inv.customer_nombre, inv.customer_address, \
         inv.customer_email, inv.customer_phone, \
         inv.huella, inv.aeat_status, inv.created_at, \
         ce.prev_hash, ce.curr_hash \
         FROM invoice inv \
         JOIN chain_entry ce ON ce.entry_type = 'UPGRADE' AND ce.entry_pk = inv.id \
         WHERE inv.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .ok_or_else(|| AppError::not_found(format!("Upgrade invoice {id} not found")))?;

    Ok(Json(UpgradeDetailResponse {
        id: row.id,
        invoice_number: row.invoice_number,
        serie: row.serie,
        tipo_factura: row.tipo_factura,
        source_pk: row.source_pk,
        subtotal: row.subtotal,
        tax: row.tax,
        total: row.total,
        factura_sustituida_id: row.factura_sustituida_id,
        factura_sustituida_num: row.factura_sustituida_num,
        customer_nif: row.customer_nif,
        customer_nombre: row.customer_nombre,
        customer_address: row.customer_address,
        customer_email: row.customer_email,
        customer_phone: row.customer_phone,
        huella: row.huella,
        aeat_status: row.aeat_status,
        created_at: row.created_at,
        prev_hash: row.prev_hash,
        curr_hash: row.curr_hash,
    }))
}
