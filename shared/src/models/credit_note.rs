//! Credit Note (退款凭证) Model

use serde::{Deserialize, Serialize};

/// Credit Note entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct CreditNote {
    pub id: i64,
    pub credit_note_number: String,
    pub original_order_pk: i64,
    pub original_receipt: String,

    // 金额（正数）
    pub subtotal_credit: f64,
    pub tax_credit: f64,
    pub total_credit: f64,

    // 退款方式
    pub refund_method: String,

    // 审计
    pub reason: String,
    pub note: Option<String>,
    pub operator_id: i64,
    pub operator_name: String,
    pub authorizer_id: Option<i64>,
    pub authorizer_name: Option<String>,

    // 归属
    pub shift_id: Option<i64>,
    pub cloud_synced: i64,
    pub created_at: i64,
}

/// Credit Note Item (退款明细行)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct CreditNoteItem {
    pub id: i64,
    pub credit_note_id: i64,
    pub original_instance_id: String,
    pub item_name: String,
    pub quantity: i64,
    pub unit_price: f64,
    pub line_credit: f64,
    pub tax_rate: i64,
    pub tax_credit: f64,
}

/// Credit Note with items (查询响应)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditNoteDetail {
    #[serde(flatten)]
    pub credit_note: CreditNote,
    pub items: Vec<CreditNoteItem>,
}

/// Refundable info for anti-over-refund
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundableInfo {
    pub original_order_pk: i64,
    pub original_receipt: String,
    pub original_total: f64,
    pub already_refunded: f64,
    pub remaining_refundable: f64,
}

/// Request to create a credit note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCreditNoteRequest {
    pub original_order_pk: i64,
    pub items: Vec<CreditNoteItemRequest>,
    pub refund_method: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizer_name: Option<String>,
}

/// Item in a credit note request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditNoteItemRequest {
    pub instance_id: String,
    pub quantity: i64,
}
