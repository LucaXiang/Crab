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
    /// Cursor: only return entries with chain_id < before (for stable pagination)
    pub before: Option<i64>,
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
    pub spec_name: Option<String>,
    pub is_comped: bool,
    pub quantity: i64,
    pub unit_price: f64,
    pub line_credit: f64,
    pub tax_rate: i64,
    pub tax_credit: f64,
}

// ── Handler: list ────────────────────────────────────────────────────────────

/// SQL: UNION ALL 查询 ORDER + CREDIT_NOTE + ANULACION + UPGRADE + BREAK，按 chain_id DESC 排序
///
/// 搜索参数需要绑定多次（SQLite 不支持参数复用），
/// ORDER/CREDIT_NOTE/ANULACION/UPGRADE/BREAK 各分支分别绑定。
///
/// ANULACION/UPGRADE entry_pk 指向 archived_order.id（订单层）。
/// BREAK entry_pk 指向失败的 chain_entry.id，不 JOIN 其他表。
const LIST_SQL: &str = "\
SELECT
    ce.id          AS chain_id,
    ce.entry_type,
    ce.entry_pk,
    COALESCE(ao.receipt_number, CAST(ao.id AS TEXT)) AS display_number,
    CASE
      WHEN ao.is_voided = 1 THEN 'ANULADA'
      WHEN ao.status = 'VOID' AND ao.void_type = 'LOSS_SETTLED' THEN 'LOSS'
      ELSE UPPER(ao.status)
    END AS status,
    ao.total_amount AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    NULL           AS original_order_pk,
    NULL           AS original_receipt
FROM chain_entry ce
JOIN archived_order ao ON ao.id = ce.entry_pk
WHERE ce.entry_type = 'ORDER'
  AND (? IS NULL OR LOWER(COALESCE(ao.receipt_number, CAST(ao.id AS TEXT))) LIKE ?)

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
    COALESCE(ao2.receipt_number, CAST(ao2.id AS TEXT)) AS display_number,
    'ANULADA'      AS status,
    ao2.total_amount AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    ao2.id         AS original_order_pk,
    ao2.receipt_number AS original_receipt
FROM chain_entry ce
JOIN archived_order ao2 ON ao2.id = ce.entry_pk
WHERE ce.entry_type = 'ANULACION'
  AND (? IS NULL OR LOWER(COALESCE(ao2.receipt_number, CAST(ao2.id AS TEXT))) LIKE ?)

UNION ALL

SELECT
    ce.id          AS chain_id,
    ce.entry_type,
    ce.entry_pk,
    COALESCE(ao3.receipt_number, CAST(ao3.id AS TEXT)) AS display_number,
    'UPGRADED'     AS status,
    ao3.total_amount AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    ao3.id         AS original_order_pk,
    ao3.receipt_number AS original_receipt
FROM chain_entry ce
JOIN archived_order ao3 ON ao3.id = ce.entry_pk
WHERE ce.entry_type = 'UPGRADE'
  AND (? IS NULL OR LOWER(COALESCE(ao3.receipt_number, CAST(ao3.id AS TEXT))) LIKE ?)

UNION ALL

SELECT
    ce.id          AS chain_id,
    ce.entry_type,
    ce.entry_pk,
    '#' || CAST(ce.id % 10000 AS TEXT) AS display_number,
    NULL           AS status,
    0.0            AS amount,
    ce.created_at,
    ce.prev_hash,
    ce.curr_hash,
    NULL           AS original_order_pk,
    NULL           AS original_receipt
FROM chain_entry ce
WHERE ce.entry_type = 'BREAK'
  AND (? IS NULL OR 'break' LIKE ?)

ORDER BY chain_id DESC";

const COUNT_SQL: &str = "\
SELECT COUNT(*) FROM (
    SELECT ce.id FROM chain_entry ce
    JOIN archived_order ao ON ao.id = ce.entry_pk
    WHERE ce.entry_type = 'ORDER'
      AND (? IS NULL OR LOWER(COALESCE(ao.receipt_number, CAST(ao.id AS TEXT))) LIKE ?)
    UNION ALL
    SELECT ce.id FROM chain_entry ce
    JOIN credit_note cn ON cn.id = ce.entry_pk
    WHERE ce.entry_type = 'CREDIT_NOTE'
      AND (? IS NULL OR LOWER(cn.credit_note_number) LIKE ?
           OR LOWER(cn.original_receipt) LIKE ?)
    UNION ALL
    SELECT ce.id FROM chain_entry ce
    JOIN archived_order ao2 ON ao2.id = ce.entry_pk
    WHERE ce.entry_type = 'ANULACION'
      AND (? IS NULL OR LOWER(COALESCE(ao2.receipt_number, CAST(ao2.id AS TEXT))) LIKE ?)
    UNION ALL
    SELECT ce.id FROM chain_entry ce
    JOIN archived_order ao3 ON ao3.id = ce.entry_pk
    WHERE ce.entry_type = 'UPGRADE'
      AND (? IS NULL OR LOWER(COALESCE(ao3.receipt_number, CAST(ao3.id AS TEXT))) LIKE ?)
    UNION ALL
    SELECT ce.id FROM chain_entry ce
    WHERE ce.entry_type = 'BREAK'
      AND (? IS NULL OR 'break' LIKE ?)
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
    let search_pattern = params
        .search
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    // COUNT — 11 binds: (2) ORDER + (3) CREDIT_NOTE + (2) ANULACION + (2) UPGRADE + (2) BREAK
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

    // Cursor-based or offset-based pagination
    let rows = if let Some(before) = params.before {
        let sql = format!(
            "SELECT * FROM ({base}) WHERE chain_id < ? LIMIT ?",
            base = LIST_SQL
        );
        sqlx::query_as::<_, ChainEntryRaw>(&sql)
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
            .bind(before)
            .bind(limit)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| AppError::database(e.to_string()))?
    } else {
        let offset = params.offset.unwrap_or(0);
        let sql = format!("{} LIMIT ? OFFSET ?", LIST_SQL);
        sqlx::query_as::<_, ChainEntryRaw>(&sql)
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
            .map_err(|e| AppError::database(e.to_string()))?
    };

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
        "SELECT cni.id, cni.original_instance_id, cni.item_name, \
         aoi.spec_name, COALESCE(aoi.is_comped, 0) AS is_comped, \
         cni.quantity, cni.unit_price, \
         cni.line_credit, cni.tax_rate, cni.tax_credit \
         FROM credit_note_item cni \
         LEFT JOIN archived_order_item aoi ON aoi.instance_id = cni.original_instance_id \
           AND aoi.order_pk = (SELECT original_order_pk FROM credit_note WHERE id = cni.credit_note_id) \
         WHERE cni.credit_note_id = ? ORDER BY cni.id",
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

/// Anulación detail response — order-layer info with chain hashes
#[derive(Debug, Serialize)]
pub struct AnulacionDetailResponse {
    pub order_pk: i64,
    pub receipt_number: String,
    pub total_amount: f64,
    pub is_voided: bool,
    pub operator_name: Option<String>,
    pub created_at: i64,
    pub prev_hash: String,
    pub curr_hash: String,
    pub items: Vec<AnulacionItemRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AnulacionItemRow {
    pub instance_id: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub quantity: i64,
    pub unit_price: f64,
    pub line_total: f64,
    pub is_comped: bool,
    pub tax_rate: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct AnulacionDetailRow {
    order_pk: i64,
    receipt_number: String,
    total_amount: f64,
    is_voided: i64,
    operator_name: Option<String>,
    created_at: i64,
    prev_hash: String,
    curr_hash: String,
}

/// GET /api/chain-entries/anulacion/:order_pk — entry_pk = order_pk
pub async fn get_anulacion_detail(
    State(state): State<ServerState>,
    Path(order_pk): Path<i64>,
) -> AppResult<Json<AnulacionDetailResponse>> {
    let row = sqlx::query_as::<_, AnulacionDetailRow>(
        "SELECT ao.id AS order_pk, \
         COALESCE(ao.receipt_number, CAST(ao.id AS TEXT)) AS receipt_number, \
         ao.total_amount, ao.is_voided, ao.operator_name, ce.created_at, \
         ce.prev_hash, ce.curr_hash \
         FROM chain_entry ce \
         JOIN archived_order ao ON ao.id = ce.entry_pk \
         WHERE ce.entry_type = 'ANULACION' AND ce.entry_pk = ?",
    )
    .bind(order_pk)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .ok_or_else(|| AppError::not_found(format!("Anulación for order {order_pk} not found")))?;

    let items = sqlx::query_as::<_, AnulacionItemRow>(
        "SELECT instance_id, name, spec_name, quantity, unit_price, line_total, \
         is_comped, tax_rate \
         FROM archived_order_item WHERE order_pk = ? ORDER BY id",
    )
    .bind(order_pk)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(AnulacionDetailResponse {
        order_pk: row.order_pk,
        receipt_number: row.receipt_number,
        total_amount: row.total_amount,
        is_voided: row.is_voided != 0,
        operator_name: row.operator_name,
        created_at: row.created_at,
        prev_hash: row.prev_hash,
        curr_hash: row.curr_hash,
        items,
    }))
}

// ── Handler: get_upgrade_detail ──────────────────────────────────────────────

/// Upgrade detail response — order-layer info with customer data and chain hashes
#[derive(Debug, Serialize)]
pub struct UpgradeDetailResponse {
    pub order_pk: i64,
    pub receipt_number: String,
    pub total_amount: f64,
    pub tax: f64,
    pub is_upgraded: bool,
    pub customer_nif: Option<String>,
    pub customer_nombre: Option<String>,
    pub customer_address: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub created_at: i64,
    pub prev_hash: String,
    pub curr_hash: String,
}

#[derive(Debug, sqlx::FromRow)]
struct UpgradeDetailRow {
    order_pk: i64,
    receipt_number: String,
    total_amount: f64,
    tax: f64,
    is_upgraded: i64,
    customer_nif: Option<String>,
    customer_nombre: Option<String>,
    customer_address: Option<String>,
    customer_email: Option<String>,
    customer_phone: Option<String>,
    created_at: i64,
    prev_hash: String,
    curr_hash: String,
}

/// GET /api/chain-entries/upgrade/:order_pk — entry_pk = order_pk
pub async fn get_upgrade_detail(
    State(state): State<ServerState>,
    Path(order_pk): Path<i64>,
) -> AppResult<Json<UpgradeDetailResponse>> {
    let row = sqlx::query_as::<_, UpgradeDetailRow>(
        "SELECT ao.id AS order_pk, \
         COALESCE(ao.receipt_number, CAST(ao.id AS TEXT)) AS receipt_number, \
         ao.total_amount, ao.tax, ao.is_upgraded, \
         ao.customer_nif, ao.customer_nombre, ao.customer_address, \
         ao.customer_email, ao.customer_phone, \
         ce.created_at, ce.prev_hash, ce.curr_hash \
         FROM chain_entry ce \
         JOIN archived_order ao ON ao.id = ce.entry_pk \
         WHERE ce.entry_type = 'UPGRADE' AND ce.entry_pk = ?",
    )
    .bind(order_pk)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?
    .ok_or_else(|| AppError::not_found(format!("Upgrade for order {order_pk} not found")))?;

    Ok(Json(UpgradeDetailResponse {
        order_pk: row.order_pk,
        receipt_number: row.receipt_number,
        total_amount: row.total_amount,
        tax: row.tax,
        is_upgraded: row.is_upgraded != 0,
        customer_nif: row.customer_nif,
        customer_nombre: row.customer_nombre,
        customer_address: row.customer_address,
        customer_email: row.customer_email,
        customer_phone: row.customer_phone,
        created_at: row.created_at,
        prev_hash: row.prev_hash,
        curr_hash: row.curr_hash,
    }))
}
