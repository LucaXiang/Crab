/**
 * Archived Order Types
 *
 * Types for archived order history from SurrealDB graph model.
 * Matches backend Rust types exactly.
 */

// ============================================================================
// List View Types
// ============================================================================

/** Order summary for list view (matches backend OrderSummary) */
export interface ArchivedOrderSummary {
  order_id: string;
  receipt_number: string;
  table_name: string | null;
  status: string;
  is_retail: boolean;
  total: number;
  guest_count: number;
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
}

/** Order item for detail view */
export interface ArchivedOrderItem {
  id: string;
  instance_id: string;
  name: string;
  spec_name: string | null;
  price: number;
  quantity: number;
  unpaid_quantity: number;
  unit_price: number;
  line_total: number;
  discount_amount: number;
  surcharge_amount: number;
  note: string | null;
  selected_options: ArchivedItemOption[];
}

/** Payment for detail view */
export interface ArchivedPayment {
  method: string;
  amount: number;
  timestamp: number; // milliseconds
  note: string | null;
  cancelled: boolean;
  cancel_reason: string | null;
  split_items: ArchivedSplitItem[];
}

/** Event for detail view */
export interface ArchivedEvent {
  event_id: string;
  event_type: string;
  timestamp: number; // milliseconds
  payload: unknown | null;
}

/** Void type for archived orders */
export type ArchivedVoidType = 'CANCELLED' | 'LOSS_SETTLED';

/** Loss reason for archived void orders */
export type ArchivedLossReason = 'CUSTOMER_FLED' | 'CUSTOMER_INSOLVENT' | 'OTHER';

/** Full order detail (matches backend OrderDetail) */
export interface ArchivedOrderDetail {
  order_id: string;
  receipt_number: string;
  table_name: string | null;
  zone_name: string | null;
  status: string;
  is_retail: boolean;
  guest_count: number;
  total: number;
  paid_amount: number;
  total_discount: number;
  total_surcharge: number;
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
