export interface ItemOption {
  attribute_id: number;
  attribute_name: string;
  option_id: number;
  option_name: string;
  price_modifier?: number;
  quantity?: number;
}

export interface SpecificationInfo {
  id: number;
  name: string;
  receipt_name?: string;
  price?: number;
  is_multi_spec?: boolean;
}

export interface PaymentRecord {
  payment_id: string;
  method: string;
  amount: number;
  tendered?: number;
  change?: number;
  note?: string;
  timestamp: number;
  cancelled: boolean;
  cancel_reason?: string;
  split_type?: string;
  aa_shares?: number;
}

export interface CartItemSnapshot {
  id: number;
  instance_id: string;
  name: string;
  price: number;
  original_price: number;
  quantity: number;
  unpaid_quantity: number;
  unit_price: number;
  line_total: number;
  tax: number;
  tax_rate: number;
  is_comped: boolean;
  selected_options?: ItemOption[];
  selected_specification?: SpecificationInfo;
  manual_discount_percent?: number;
  rule_discount_amount: number;
  rule_surcharge_amount: number;
  mg_discount_amount: number;
  note?: string;
  authorizer_name?: string;
  category_name?: string;
}

export interface OrderEvent {
  event_id: string;
  sequence: number;
  order_id: string;
  timestamp: number;
  client_timestamp?: number;
  operator_id: number;
  operator_name: string;
  command_id: string;
  event_type: string;
  payload: Record<string, unknown>;
}

export interface LiveOrderSnapshot {
  edge_server_id: number;
  order_id: string;
  table_id?: number;
  table_name?: string;
  zone_id?: number;
  zone_name?: string;
  guest_count: number;
  is_retail: boolean;
  service_type?: string;
  queue_number?: number;
  status: string;
  items: CartItemSnapshot[];
  payments: PaymentRecord[];
  original_total: number;
  subtotal: number;
  total_discount: number;
  total_surcharge: number;
  tax: number;
  total: number;
  paid_amount: number;
  remaining_amount: number;
  comp_total_amount: number;
  order_manual_discount_amount: number;
  order_manual_surcharge_amount: number;
  order_rule_discount_amount: number;
  order_rule_surcharge_amount: number;
  receipt_number: string;
  note?: string;
  created_at: number;
  updated_at: number;
  start_time: number;
  operator_id?: number;
  operator_name?: string;
  member_id?: number;
  member_name?: string;
  marketing_group_name?: string;
  events?: OrderEvent[];
}

export type ConnectionState = 'connecting' | 'connected' | 'reconnecting' | 'disconnected';

export type ConsoleMessage =
  | { type: 'Ready'; snapshots: LiveOrderSnapshot[]; online_edge_ids?: number[] }
  | { type: 'OrderUpdated'; snapshot: LiveOrderSnapshot }
  | { type: 'OrderRemoved'; order_id: string; edge_server_id: number }
  | { type: 'EdgeStatus'; edge_server_id: number; online: boolean; cleared_order_ids?: string[] };
