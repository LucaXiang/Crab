/**
 * Chain Entry Types
 *
 * 统一 hash 链时间线类型，与 edge-server chain_entries API 对齐。
 */

export type ChainEntryType = 'ORDER' | 'CREDIT_NOTE' | 'ANULACION' | 'UPGRADE' | 'BREAK';

export type ChainEntryStatus = 'VOID' | 'LOSS' | 'MERGED' | 'ANULADA' | 'COMPLETED' | null;

/** chain_entry 列表项 (GET /api/chain-entries) */
export interface ChainEntryItem {
  chain_id: number;
  entry_type: ChainEntryType;
  entry_pk: number;
  display_number: string;
  status: ChainEntryStatus;
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
  spec_name: string | null;
  is_comped: boolean;
  quantity: number;
  unit_price: number;
  line_credit: number;
  tax_rate: number;
  tax_credit: number;
}

/** 作废条目的原始订单菜品 */
export interface ChainAnulacionItem {
  instance_id: string;
  name: string;
  spec_name: string | null;
  quantity: number;
  unit_price: number;
  line_total: number;
  is_comped: boolean;
  tax_rate: number;
}

/** 作废详情 (GET /api/chain-entries/anulacion/:order_pk) */
export interface ChainAnulacionDetail {
  order_pk: number;
  receipt_number: string;
  total_amount: number;
  is_voided: boolean;
  operator_name: string | null;
  created_at: number;
  prev_hash: string;
  curr_hash: string;
  items: ChainAnulacionItem[];
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

/** 升级详情 (GET /api/chain-entries/upgrade/:order_pk) */
export interface ChainUpgradeDetail {
  order_pk: number;
  receipt_number: string;
  total_amount: number;
  tax: number;
  is_upgraded: boolean;
  customer_nif: string | null;
  customer_nombre: string | null;
  customer_address: string | null;
  customer_email: string | null;
  customer_phone: string | null;
  created_at: number;
  prev_hash: string;
  curr_hash: string;
}
