export interface OrderSummary {
  id: number;
  source_id: string;
  receipt_number: string | null;
  status: string;
  end_time: number | null;
  total: number | null;
  synced_at: number;
}

export type ChainEntryType = 'ORDER' | 'CREDIT_NOTE' | 'ANULACION' | 'UPGRADE';

export interface ChainEntryItem {
  entry_type: ChainEntryType;
  entry_id: number;
  display_number: string;
  status: string;
  amount: number | null;
  created_at: number;
  original_order_id: number | null;
  original_receipt: string | null;
}

export interface AnulacionDetailResponse {
  order_id: number;
  receipt_number: string;
  total_amount: number;
  is_anulada: boolean;
  created_at: number;
}

export interface UpgradeDetailResponse {
  order_id: number;
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
}

export interface CreditNoteDetailResponse {
  source_id: number;
  credit_note_number: string;
  original_order_id: number;
  original_receipt: string;
  subtotal_credit: number;
  tax_credit: number;
  total_credit: number;
  refund_method: string;
  reason: string;
  note: string | null;
  operator_name: string;
  authorizer_name: string | null;
  created_at: number;
  items: CreditNoteItem[];
}

export interface CreditNoteItem {
  item_name: string;
  quantity: number;
  unit_price: number;
  line_credit: number;
  tax_rate: number;
  tax_credit: number;
}

export interface OrderItemOption {
  attribute_name: string;
  option_name: string;
  price: number;
  quantity: number;
}

export interface OrderItem {
  name: string;
  spec_name: string | null;
  category_name: string | null;
  price: number;
  quantity: number;
  unit_price: number;
  line_total: number;
  discount_amount: number;
  surcharge_amount: number;
  tax: number;
  tax_rate: number;
  is_comped: boolean;
  note: string | null;
  options: OrderItemOption[];
}

export interface OrderPayment {
  seq: number;
  method: string;
  amount: number;
  timestamp: number;
  cancelled: boolean;
}

export interface TaxDesglose {
  tax_rate: number;
  base_amount: number;
  tax_amount: number;
}

export interface OrderEvent {
  seq: number;
  event_type: string;
  timestamp: number;
  operator_id: number | null;
  operator_name: string | null;
  data: string | null;
}

export interface OrderDetailPayload {
  zone_name: string | null;
  table_name: string | null;
  is_retail: boolean;
  guest_count: number | null;
  original_total: number;
  subtotal: number;
  paid_amount: number;
  discount_amount: number;
  surcharge_amount: number;
  comp_total_amount: number;
  order_manual_discount_amount: number;
  order_manual_surcharge_amount: number;
  order_rule_discount_amount: number;
  order_rule_surcharge_amount: number;
  start_time: number;
  operator_name: string | null;
  void_type: string | null;
  loss_reason: string | null;
  loss_amount: number | null;
  void_note: string | null;
  member_name: string | null;
  items: OrderItem[];
  payments: OrderPayment[];
  events?: OrderEvent[];
}

export interface OrderDetailResponse {
  source: string;
  detail: OrderDetailPayload;
  desglose: TaxDesglose[];
}

export interface CreditNoteSummary {
  source_id: number;
  credit_note_number: string;
  total_credit: number;
  refund_method: string;
  reason: string;
  operator_name: string;
  created_at: number;
}
