/**
 * Chain Entry Types
 *
 * 统一 hash 链时间线类型，与 edge-server chain_entries API 对齐。
 */

export type ChainEntryType = 'ORDER' | 'CREDIT_NOTE' | 'ANULACION' | 'UPGRADE';

/** chain_entry 列表项 (GET /api/chain-entries) */
export interface ChainEntryItem {
  chain_id: number;
  entry_type: ChainEntryType;
  entry_pk: number;
  display_number: string;
  status: string | null;
  amount: number;
  created_at: number;
  prev_hash: string;
  curr_hash: string;
  original_order_pk: number | null;
  original_receipt: string | null;
}

/** GET /api/chain-entries 分页响应 */
export interface ChainEntryListResponse {
  entries: ChainEntryItem[];
  total: number;
}

/** 退款凭证明细行 */
export interface ChainCreditNoteItem {
  id: number;
  original_instance_id: string;
  item_name: string;
  quantity: number;
  unit_price: number;
  line_credit: number;
  tax_rate: number;
  tax_credit: number;
}

/** 发票作废详情 (GET /api/chain-entries/anulacion/:id) */
export interface ChainAnulacionDetail {
  id: number;
  anulacion_number: string;
  serie: string;
  original_invoice_id: number;
  original_invoice_number: string;
  original_order_pk: number;
  reason: string;
  note: string | null;
  operator_id: number;
  operator_name: string;
  huella: string;
  aeat_status: string;
  created_at: number;
  prev_hash: string;
  curr_hash: string;
}

/** 退款凭证详情 (GET /api/chain-entries/credit-note/:id) */
export interface ChainCreditNoteDetail {
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
  created_at: number;
  prev_hash: string;
  curr_hash: string;
  items: ChainCreditNoteItem[];
}

/** F3 发票升级详情 (GET /api/chain-entries/upgrade/:id) */
export interface ChainUpgradeDetail {
  id: number;
  invoice_number: string;
  serie: string;
  tipo_factura: string;
  source_pk: number;
  subtotal: number;
  tax: number;
  total: number;
  factura_sustituida_id: number | null;
  factura_sustituida_num: string | null;
  customer_nif: string | null;
  customer_nombre: string | null;
  customer_address: string | null;
  customer_email: string | null;
  customer_phone: string | null;
  huella: string;
  aeat_status: string;
  created_at: number;
  prev_hash: string;
  curr_hash: string;
}
