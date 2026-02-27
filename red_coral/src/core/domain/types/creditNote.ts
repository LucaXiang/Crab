/**
 * Credit Note Types
 *
 * 退款凭证类型定义，与 Rust shared::models 对齐。
 */

/** Credit note summary (matches backend CreditNote) */
export interface CreditNote {
  id: number;
  credit_note_number: string;
  original_order_pk: number;
  original_receipt: string;
  subtotal_credit: number;
  tax_credit: number;
  total_credit: number;
  refund_method: string;
  reason: string;
  note: string | null;
  operator_id: number;
  operator_name: string;
  authorizer_id: number | null;
  authorizer_name: string | null;
  shift_id: number | null;
  cloud_synced: number;
  created_at: number;
}

/** Credit note item (matches backend CreditNoteItem) */
export interface CreditNoteItem {
  id: number;
  credit_note_id: number;
  original_instance_id: string;
  item_name: string;
  quantity: number;
  unit_price: number;
  line_credit: number;
  tax_rate: number;
  tax_credit: number;
}

/** Credit note with items (matches backend CreditNoteDetail) */
export interface CreditNoteDetail extends CreditNote {
  items: CreditNoteItem[];
}

/** Refundable info for anti-over-refund (matches backend RefundableInfo) */
export interface RefundableInfo {
  original_order_pk: number;
  original_receipt: string;
  original_total: number;
  already_refunded: number;
  remaining_refundable: number;
}

/** Request to create a credit note */
export interface CreateCreditNoteRequest {
  original_order_pk: number;
  items: CreditNoteItemRequest[];
  refund_method: string;
  reason: string;
  note?: string | null;
  authorizer_id?: number | null;
  authorizer_name?: string | null;
}

/** Item in a credit note creation request */
export interface CreditNoteItemRequest {
  instance_id: string;
  quantity: number;
}
