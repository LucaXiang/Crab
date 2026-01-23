/**
 * Order Event System Types
 *
 * TypeScript type definitions for the order event sourcing system.
 * These types match the Rust types in `shared/src/order/` and the JSON Schema.
 *
 * Architecture:
 * - Commands: Frontend -> Backend (intent)
 * - Events: Backend -> Frontend (facts via broadcast)
 * - Snapshots: Computed state from events
 */

// ============================================================================
// Event Types
// ============================================================================

/**
 * All order event types (matches Rust OrderEventType)
 */
export type OrderEventType =
  | 'TABLE_OPENED'
  | 'ORDER_COMPLETED'
  | 'ORDER_VOIDED'
  | 'ORDER_RESTORED'
  | 'ITEMS_ADDED'
  | 'ITEM_MODIFIED'
  | 'ITEM_REMOVED'
  | 'ITEM_RESTORED'
  | 'PAYMENT_ADDED'
  | 'PAYMENT_CANCELLED'
  | 'ORDER_SPLIT'
  | 'ORDER_MOVED'
  | 'ORDER_MOVED_OUT'
  | 'ORDER_MERGED'
  | 'ORDER_MERGED_OUT'
  | 'TABLE_REASSIGNED'
  | 'SURCHARGE_EXEMPT_SET'
  | 'ORDER_INFO_UPDATED'
  | 'RULE_SKIP_TOGGLED';

/**
 * Order event structure (matches Rust OrderEvent)
 *
 * IMPORTANT: Clock Skew Handling
 * - `timestamp` is SERVER time (authoritative for state evolution)
 * - `client_timestamp` is CLIENT time (for audit/debugging only)
 * - State ordering MUST use `sequence`, NOT timestamps
 */
export interface OrderEvent {
  /** Event unique ID (UUID) */
  event_id: string;
  /** Global incrementing sequence number for ordering/replay - AUTHORITATIVE for state evolution */
  sequence: number;
  /** Order this event belongs to */
  order_id: string;
  /** Server timestamp (Unix milliseconds) - AUTHORITATIVE, always set by server */
  timestamp: number;
  /** Client timestamp (Unix milliseconds) - for audit/debugging, may differ due to clock skew */
  client_timestamp?: number | null;
  /** Operator who performed the action */
  operator_id: string;
  /** Operator name snapshot (for timeline display) */
  operator_name: string;
  /** Command ID that triggered this event (audit trail) */
  command_id: string;
  /** Event type */
  event_type: OrderEventType;
  /** Event payload (varies by type) */
  payload: EventPayload;
}

// ============================================================================
// Event Payloads
// ============================================================================

export type EventPayload =
  | TableOpenedPayload
  | OrderCompletedPayload
  | OrderVoidedPayload
  | OrderRestoredPayload
  | ItemsAddedPayload
  | ItemModifiedPayload
  | ItemRemovedPayload
  | ItemRestoredPayload
  | PaymentAddedPayload
  | PaymentCancelledPayload
  | OrderSplitPayload
  | OrderMovedPayload
  | OrderMovedOutPayload
  | OrderMergedPayload
  | OrderMergedOutPayload
  | TableReassignedPayload
  | OrderInfoUpdatedPayload
  | RuleSkipToggledPayload;

export interface TableOpenedPayload {
  type: 'TABLE_OPENED';
  table_id: string | null;
  table_name: string | null;
  zone_id: string | null;
  zone_name: string | null;
  guest_count: number;
  is_retail: boolean;
  receipt_number?: string | null;
}

export interface OrderCompletedPayload {
  type: 'ORDER_COMPLETED';
  receipt_number: string;
  final_total: number;
  payment_summary: PaymentSummaryItem[];
}

export interface OrderVoidedPayload {
  type: 'ORDER_VOIDED';
  reason?: string | null;
}

export interface OrderRestoredPayload {
  type: 'ORDER_RESTORED';
}

export interface ItemsAddedPayload {
  type: 'ITEMS_ADDED';
  items: CartItemSnapshot[];
}

export interface ItemModifiedPayload {
  type: 'ITEM_MODIFIED';
  /** Operation description for audit */
  operation: string;
  /** Source item before modification */
  source: CartItemSnapshot;
  /** Number of items affected */
  affected_quantity: number;
  /** Changes applied */
  changes: ItemChanges;
  /** Previous values for comparison */
  previous_values: ItemChanges;
  /** Resulting items after modification */
  results: ItemModificationResult[];
  /** Authorizer ID */
  authorizer_id?: string | null;
  /** Authorizer name */
  authorizer_name?: string | null;
}

export interface ItemModificationResult {
  instance_id: string;
  quantity: number;
  price: number;
  manual_discount_percent?: number | null;
  action: string;
}

export interface ItemRemovedPayload {
  type: 'ITEM_REMOVED';
  instance_id: string;
  item_name: string;
  quantity?: number | null;
  reason?: string | null;
}

export interface ItemRestoredPayload {
  type: 'ITEM_RESTORED';
  instance_id: string;
  item_name: string;
}

export interface PaymentAddedPayload {
  type: 'PAYMENT_ADDED';
  payment_id: string;
  method: PaymentMethod;
  amount: number;
  tendered?: number | null;
  change?: number | null;
  note?: string | null;
}

export interface PaymentCancelledPayload {
  type: 'PAYMENT_CANCELLED';
  payment_id: string;
  method: string;
  amount: number;
  reason?: string | null;
}

export interface OrderSplitPayload {
  type: 'ORDER_SPLIT';
  split_amount: number;
  payment_method: string;
  items: SplitItem[];
}

export interface OrderMovedPayload {
  type: 'ORDER_MOVED';
  source_table_id: string;
  source_table_name: string;
  target_table_id: string;
  target_table_name: string;
  items: CartItemSnapshot[];
}

export interface OrderMovedOutPayload {
  type: 'ORDER_MOVED_OUT';
  target_table_id: string;
  target_table_name: string;
  reason?: string | null;
}

export interface OrderMergedPayload {
  type: 'ORDER_MERGED';
  source_table_id: string;
  source_table_name: string;
  items: CartItemSnapshot[];
}

export interface OrderMergedOutPayload {
  type: 'ORDER_MERGED_OUT';
  target_table_id: string;
  target_table_name: string;
  reason?: string | null;
}

export interface TableReassignedPayload {
  type: 'TABLE_REASSIGNED';
  source_table_id: string;
  source_table_name: string;
  target_table_id: string;
  target_table_name: string;
  target_zone_name?: string | null;
  original_start_time: number;
  items: CartItemSnapshot[];
}

export interface OrderInfoUpdatedPayload {
  type: 'ORDER_INFO_UPDATED';
  receipt_number?: string | null;
  guest_count?: number | null;
  table_name?: string | null;
  is_pre_payment?: boolean | null;
}

export interface RuleSkipToggledPayload {
  type: 'RULE_SKIP_TOGGLED';
  rule_id: string;
  skipped: boolean;
  /** Recalculated subtotal */
  subtotal: number;
  /** Recalculated discount */
  discount: number;
  /** Recalculated surcharge */
  surcharge: number;
  /** Recalculated total */
  total: number;
}

// ============================================================================
// Command Types
// ============================================================================

/**
 * Order command structure (Frontend -> Backend)
 */
export interface OrderCommand {
  /** Idempotency ID (generated by frontend) */
  command_id: string;
  /** Send timestamp */
  timestamp: number;
  /** Operator ID */
  operator_id: string;
  /** Operator name */
  operator_name: string;
  /** Command payload */
  payload: OrderCommandPayload;
}

export type OrderCommandPayload =
  | OpenTableCommand
  | CompleteOrderCommand
  | VoidOrderCommand
  | RestoreOrderCommand
  | AddItemsCommand
  | ModifyItemCommand
  | RemoveItemCommand
  | RestoreItemCommand
  | AddPaymentCommand
  | CancelPaymentCommand
  | SplitOrderCommand
  | MoveOrderCommand
  | MergeOrdersCommand
  | UpdateOrderInfoCommand
  | ToggleRuleSkipCommand;

export interface OpenTableCommand {
  type: 'OPEN_TABLE';
  table_id?: string | null;
  table_name?: string | null;
  zone_id?: string | null;
  zone_name?: string | null;
  guest_count?: number;
  is_retail: boolean;
}

export interface CompleteOrderCommand {
  type: 'COMPLETE_ORDER';
  order_id: string;
  receipt_number: string;
}

export interface VoidOrderCommand {
  type: 'VOID_ORDER';
  order_id: string;
  reason?: string | null;
}

export interface RestoreOrderCommand {
  type: 'RESTORE_ORDER';
  order_id: string;
}

export interface AddItemsCommand {
  type: 'ADD_ITEMS';
  order_id: string;
  items: CartItemInput[];
}

export interface ModifyItemCommand {
  type: 'MODIFY_ITEM';
  order_id: string;
  instance_id: string;
  changes: ItemChanges;
}

export interface RemoveItemCommand {
  type: 'REMOVE_ITEM';
  order_id: string;
  instance_id: string;
  quantity?: number | null;
  reason?: string | null;
}

export interface RestoreItemCommand {
  type: 'RESTORE_ITEM';
  order_id: string;
  instance_id: string;
}

/** Payment input for AddPayment command (matches Rust PaymentInput) */
export interface PaymentInput {
  method: string;
  amount: number;
  tendered?: number | null;
  note?: string | null;
}

export interface AddPaymentCommand {
  type: 'ADD_PAYMENT';
  order_id: string;
  payment: PaymentInput;
}

export interface CancelPaymentCommand {
  type: 'CANCEL_PAYMENT';
  order_id: string;
  payment_id: string;
  reason?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

export interface SplitOrderCommand {
  type: 'SPLIT_ORDER';
  order_id: string;
  split_amount: number;
  payment_method: string;
  items: SplitItem[];
}

export interface MoveOrderCommand {
  type: 'MOVE_ORDER';
  order_id: string;
  target_table_id: string;
  target_table_name: string;
  target_zone_name?: string | null;
}

export interface MergeOrdersCommand {
  type: 'MERGE_ORDERS';
  source_order_id: string;
  target_order_id: string;
}

export interface UpdateOrderInfoCommand {
  type: 'UPDATE_ORDER_INFO';
  order_id: string;
  receipt_number?: string | null;
  guest_count?: number | null;
  table_name?: string | null;
  is_pre_payment?: boolean | null;
}

export interface ToggleRuleSkipCommand {
  type: 'TOGGLE_RULE_SKIP';
  order_id: string;
  rule_id: string;
  skipped: boolean;
}

// ============================================================================
// Response Types
// ============================================================================

/**
 * Command response (ACK/NACK)
 */
export interface CommandResponse {
  command_id: string;
  success: boolean;
  /** New order ID (only for OpenTable command) */
  order_id?: string | null;
  error?: CommandError | null;
}

export interface CommandError {
  code: CommandErrorCode;
  message: string;
}

export type CommandErrorCode =
  | 'ORDER_NOT_FOUND'
  | 'ORDER_ALREADY_COMPLETED'
  | 'ORDER_ALREADY_VOIDED'
  | 'ITEM_NOT_FOUND'
  | 'PAYMENT_NOT_FOUND'
  | 'INSUFFICIENT_QUANTITY'
  | 'INVALID_AMOUNT'
  | 'DUPLICATE_COMMAND'
  | 'INTERNAL_ERROR'
  | 'INVALID_OPERATION'
  | 'TABLE_OCCUPIED';

// ============================================================================
// Sync Types
// ============================================================================

/**
 * Reconnection sync request
 */
export interface SyncRequest {
  /** Client's last known event sequence number */
  since_sequence: number;
}

/**
 * Reconnection sync response
 */
export interface SyncResponse {
  /** Incremental events since since_sequence */
  events: OrderEvent[];
  /** All active order snapshots */
  active_orders: OrderSnapshot[];
  /** Server's current sequence number */
  server_sequence: number;
  /** Whether full sync is required (gap too large) */
  requires_full_sync: boolean;
  /**
   * Server instance epoch (UUID generated on startup)
   * Used to detect server restarts - if epoch changes, client MUST full sync
   * regardless of sequence gap, as server's in-memory state was reset.
   */
  server_epoch: string;
}

// ============================================================================
// Snapshot Types
// ============================================================================

/**
 * Order status
 */
export type OrderStatus = 'ACTIVE' | 'COMPLETED' | 'VOID' | 'MOVED' | 'MERGED';

/**
 * Order snapshot (computed from event stream)
 *
 * IMPORTANT: Drift Detection
 * The `state_checksum` field can be used to detect if the frontend reducer
 * has diverged from the server's authoritative state. If checksums don't match,
 * the client should request a full snapshot refresh.
 */
export interface OrderSnapshot {
  order_id: string;
  table_id: string | null;
  table_name: string | null;
  zone_id: string | null;
  zone_name: string | null;
  guest_count: number;
  is_retail: boolean;
  status: OrderStatus;
  items: CartItemSnapshot[];
  payments: PaymentRecord[];

  // === Financial Totals (all computed by server) ===
  /** Original total before any discounts/surcharges */
  original_total: number;
  /** Subtotal after item-level adjustments (before order-level adjustments) */
  subtotal: number;
  /** Total discount amount (item-level + order-level) */
  total_discount: number;
  /** Total surcharge amount (item-level + order-level) */
  total_surcharge: number;
  tax: number;
  /** Legacy discount field (use total_discount instead) */
  discount: number;
  /** Total amount to pay */
  total: number;
  /** Amount already paid */
  paid_amount: number;
  /** Remaining amount to pay (total - paid_amount) */
  remaining_amount: number;
  /** Quantities paid per item (for split bill) */
  paid_item_quantities?: Record<string, number>;
  receipt_number: string | null;
  is_pre_payment?: boolean;
  start_time: number;
  end_time: number | null;
  created_at: number;
  updated_at: number;
  /** Last applied event sequence number */
  last_sequence: number;
  /**
   * State checksum for drift detection (16-char hex string)
   * Computed from: items.len, total(cents), paid_amount(cents), last_sequence, status
   * If local checksum != server checksum, trigger full snapshot sync
   */
  state_checksum?: string;
}

// ============================================================================
// Shared Types
// ============================================================================

export type PaymentMethod = 'cash' | 'card' | 'wechat' | 'alipay' | 'other';

/**
 * Cart item snapshot (for events and snapshots)
 */
export interface CartItemSnapshot {
  /** Product ID */
  id: string;
  /** Instance ID (unique identifier for this item) */
  instance_id: string;
  name: string;
  /** Final price after all discounts */
  price: number;
  original_price?: number | null;
  quantity: number;
  /** Unpaid quantity (computed by backend: quantity - paid_quantity) */
  unpaid_quantity: number;
  selected_options?: ItemOption[] | null;
  selected_specification?: SpecificationInfo | null;

  // === Manual Adjustment ===
  /** Manual discount percentage (0-100) */
  manual_discount_percent?: number | null;
  /** Manual surcharge amount */
  surcharge?: number | null;

  // === Rule Adjustments ===
  /** Rule discount amount (calculated from price rules) */
  rule_discount_amount?: number | null;
  /** Rule surcharge amount (calculated from price rules) */
  rule_surcharge_amount?: number | null;
  /** Applied price rules list */
  applied_rules?: AppliedRule[] | null;

  // === Computed Fields ===
  /** Unit price for display (computed by backend: price with manual discount and surcharge) */
  unit_price?: number | null;
  /** Line total (computed by backend: unit_price * quantity) */
  line_total?: number | null;

  note?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
  /** Internal: marks item as removed for soft delete */
  _removed?: boolean;
}

/**
 * Applied price rule (matches Rust AppliedRule)
 */
export interface AppliedRule {
  rule_id: string;
  name: string;
  display_name: string;
  receipt_name: string;
  rule_type: 'discount' | 'surcharge';
  adjustment_type: 'percentage' | 'fixed';
  product_scope: 'global' | 'category' | 'product';
  /** Zone scope: "zone:all", "zone:retail", or specific zone ID */
  zone_scope: string;
  adjustment_value: number;
  calculated_amount: number;
  priority: number;
  is_stackable: boolean;
  is_exclusive: boolean;
  skipped: boolean;
}

/**
 * Cart item input (for AddItems command - no instance_id, generated by backend)
 */
export interface CartItemInput {
  product_id: string;
  name: string;
  price: number;
  original_price?: number | null;
  quantity: number;
  selected_options?: ItemOption[] | null;
  selected_specification?: SpecificationInfo | null;
  /** Manual discount percentage (0-100) */
  manual_discount_percent?: number | null;
  /** Manual surcharge amount */
  surcharge?: number | null;
  note?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

export interface ItemOption {
  attribute_id: string;
  attribute_name: string;
  option_idx: number;
  option_name: string;
  price_modifier?: number | null;
}

export interface SpecificationInfo {
  id: string;
  name: string;
  receipt_name?: string | null;
  price?: number | null;
}

export interface ItemChanges {
  price?: number | null;
  quantity?: number | null;
  /** Manual discount percentage (0-100) */
  manual_discount_percent?: number | null;
  surcharge?: number | null;
  note?: string | null;
  selected_options?: ItemOption[] | null;
  selected_specification?: SpecificationInfo | null;
}

export interface SplitItem {
  instance_id: string;
  name: string;
  quantity: number;
}

export interface PaymentSummaryItem {
  method: string;
  amount: number;
}

export interface PaymentRecord {
  payment_id: string;
  method: string;
  amount: number;
  tendered?: number | null;
  change?: number | null;
  note?: string | null;
  timestamp: number;
  cancelled?: boolean;
  cancel_reason?: string | null;
  /** Split payment items snapshot (for restoration on cancel) */
  split_items?: CartItemSnapshot[] | null;
}

// ============================================================================
// Connection State Type
// ============================================================================

export type OrderConnectionState = 'connected' | 'disconnected' | 'syncing';
