/**
 * Archived Order Types
 *
 * Types for archived order history.
 * Matches backend Rust types exactly.
 */

import type { AppliedRule, EventPayload } from './orderEvent';

// ============================================================================
// List View Types
// ============================================================================

/** Order summary for list view (matches backend OrderSummary) */
export interface ArchivedOrderSummary {
  order_id: number;
  receipt_number: string;
  table_name: string | null;
  status: string;
  is_retail: boolean;
  total: number;
  guest_count: number | null;
  start_time: number; // milliseconds
  end_time: number | null; // milliseconds
  // === Void Metadata ===
  void_type: ArchivedVoidType | null;
  loss_reason: ArchivedLossReason | null;
  loss_amount: number | null;
}

/** Response from fetch_order_list */
export interface ArchivedOrderListResponse {
  orders: ArchivedOrderSummary[];
  total: number;
  page: number;
}

// ============================================================================
// Detail View Types
// ============================================================================

/** Split item in a payment */
export interface ArchivedSplitItem {
  instance_id: string;
  name: string;
  quantity: number;
  unit_price: number;
}

/** Order item option for detail view */
export interface ArchivedItemOption {
  attribute_name: string;
  option_name: string;
  price_modifier: number;
  quantity: number;
}

/** Order item for detail view */
export interface ArchivedOrderItem {
  id: number;
  instance_id: string;
  name: string;
  spec_name: string | null;
  category_id: number | null;
  category_name: string | null;
  price: number;
  quantity: number;
  unpaid_quantity: number;
  unit_price: number;
  line_total: number;
  discount_amount: number;
  surcharge_amount: number;
  rule_discount_amount: number;
  rule_surcharge_amount: number;
  applied_rules: AppliedRule[] | null;
  note: string | null;
  is_comped: boolean;
  tax: number;
  tax_rate: number;
  selected_options: ArchivedItemOption[];
}

/** Payment for detail view */
export type SplitType = 'ITEM_SPLIT' | 'AMOUNT_SPLIT' | 'AA_SPLIT';

export interface ArchivedPayment {
  payment_id?: string | null;
  method: string;
  amount: number;
  timestamp: number; // milliseconds
  cancelled: boolean;
  cancel_reason: string | null;
  tendered: number | null;
  change_amount: number | null;
  split_type?: SplitType | null;
  split_items: ArchivedSplitItem[];
  aa_shares?: number | null;
  aa_total_shares?: number | null;
}

/** Event for detail view */
export interface ArchivedEvent {
  event_id: number;
  event_type: string;
  timestamp: number; // milliseconds
  payload: EventPayload | null;
}

/** Void type for archived orders */
export type ArchivedVoidType = 'CANCELLED' | 'LOSS_SETTLED';

/** Loss reason for archived void orders */
export type ArchivedLossReason = 'CUSTOMER_FLED' | 'REFUSED_TO_PAY' | 'OTHER';

/** Full order detail (matches backend OrderDetail) */
export interface ArchivedOrderDetail {
  order_id: number;
  receipt_number: string;
  table_name: string | null;
  zone_name: string | null;
  status: string;
  is_retail: boolean;
  guest_count: number | null;
  total: number;
  paid_amount: number;
  total_discount: number;
  total_surcharge: number;
  comp_total_amount: number;
  order_manual_discount_amount: number;
  order_manual_surcharge_amount: number;
  order_rule_discount_amount: number;
  order_rule_surcharge_amount: number;
  start_time: number; // milliseconds
  end_time: number | null; // milliseconds
  operator_name: string | null;
  // === Void Metadata ===
  void_type: ArchivedVoidType | null;
  loss_reason: ArchivedLossReason | null;
  loss_amount: number | null;
  items: ArchivedOrderItem[];
  payments: ArchivedPayment[];
  timeline: ArchivedEvent[];
}
