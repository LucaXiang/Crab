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
// Service Type (零售订单的服务类型)
// ============================================================================

export type ServiceType = 'DINE_IN' | 'TAKEOUT';

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
  | 'ITEMS_ADDED'
  | 'ITEM_MODIFIED'
  | 'ITEM_REMOVED'
  | 'ITEM_COMPED'
  | 'ITEM_UNCOMPED'
  | 'PAYMENT_ADDED'
  | 'PAYMENT_CANCELLED'
  | 'ITEM_SPLIT'
  | 'AMOUNT_SPLIT'
  | 'AA_SPLIT_STARTED'
  | 'AA_SPLIT_PAID'
  | 'AA_SPLIT_CANCELLED'
  | 'ORDER_MOVED'
  | 'ORDER_MOVED_OUT'
  | 'ORDER_MERGED'
  | 'ORDER_MERGED_OUT'
  | 'TABLE_REASSIGNED'
  | 'ORDER_INFO_UPDATED'
  | 'RULE_SKIP_TOGGLED'
  | 'ORDER_DISCOUNT_APPLIED'
  | 'ORDER_SURCHARGE_APPLIED'
  | 'ORDER_NOTE_ADDED';

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
  | ItemsAddedPayload
  | ItemModifiedPayload
  | ItemRemovedPayload
  | ItemCompedPayload
  | ItemUncompedPayload
  | PaymentAddedPayload
  | PaymentCancelledPayload
  | ItemSplitPayload
  | AmountSplitPayload
  | AaSplitStartedPayload
  | AaSplitPaidPayload
  | AaSplitCancelledPayload
  | OrderMovedPayload
  | OrderMovedOutPayload
  | OrderMergedPayload
  | OrderMergedOutPayload
  | TableReassignedPayload
  | OrderInfoUpdatedPayload
  | RuleSkipToggledPayload
  | OrderDiscountAppliedPayload
  | OrderSurchargeAppliedPayload
  | OrderNoteAddedPayload;

export interface TableOpenedPayload {
  type: 'TABLE_OPENED';
  table_id: string | null;
  table_name: string | null;
  zone_id: string | null;
  zone_name: string | null;
  guest_count: number;
  is_retail: boolean;
  /** 叫号（服务器生成，零售订单使用） */
  queue_number?: number | null;
  /** Server-generated receipt number (always present) */
  receipt_number: string;
}

export interface OrderCompletedPayload {
  type: 'ORDER_COMPLETED';
  receipt_number: string;
  /** 服务类型（堂食/外卖，结单时确认，仅零售订单） */
  service_type: ServiceType | null;
  final_total: number;
  payment_summary: PaymentSummaryItem[];
}

/** 作废类型 */
export type VoidType = 'CANCELLED' | 'LOSS_SETTLED';

/** 损失原因（预设选项） */
export type LossReason = 'CUSTOMER_FLED' | 'CUSTOMER_INSOLVENT' | 'OTHER';

export interface OrderVoidedPayload {
  type: 'ORDER_VOIDED';
  /** 作废类型（默认 CANCELLED） */
  void_type?: VoidType;
  /** 损失原因（仅 LOSS_SETTLED 时使用） */
  loss_reason?: LossReason | null;
  /** 损失金额（仅 LOSS_SETTLED 时使用，用于报税） */
  loss_amount?: number | null;
  /** 备注 */
  note?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
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
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

/** Item comped (gifted) - marked as free with audit trail */
export interface ItemCompedPayload {
  type: 'ITEM_COMPED';
  /** Derived instance_id (full comp: same as source, partial: {source}::comp::{uuid}) */
  instance_id: string;
  /** Source item's instance_id (for deterministic replay) */
  source_instance_id: string;
  item_name: string;
  quantity: number;
  /** Original price before comp (captured from item before zeroing) */
  original_price: number;
  reason: string;
  authorizer_id: string;
  authorizer_name: string;
}

/** Item uncomped - comp reversed, price restored */
export interface ItemUncompedPayload {
  type: 'ITEM_UNCOMPED';
  instance_id: string;
  item_name: string;
  /** Price to restore */
  restored_price: number;
  /** If set, merge comped qty back into this source item */
  merged_into?: string | null;
  authorizer_id: string;
  authorizer_name: string;
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
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

/** 菜品分单 */
export interface ItemSplitPayload {
  type: 'ITEM_SPLIT';
  payment_id: string;
  split_amount: number;
  payment_method: string;
  items: SplitItem[];
  tendered?: number | null;
  change?: number | null;
}

/** 金额分单 */
export interface AmountSplitPayload {
  type: 'AMOUNT_SPLIT';
  payment_id: string;
  split_amount: number;
  payment_method: string;
  tendered?: number | null;
  change?: number | null;
}

/** AA 开始（锁人数，记录每份金额） */
export interface AaSplitStartedPayload {
  type: 'AA_SPLIT_STARTED';
  total_shares: number;
  per_share_amount: number;
  order_total: number;
}

/** AA 支付（进度） */
export interface AaSplitPaidPayload {
  type: 'AA_SPLIT_PAID';
  payment_id: string;
  shares: number;
  amount: number;
  payment_method: string;
  progress_paid: number;
  progress_total: number;
  tendered?: number | null;
  change?: number | null;
}

/** AA 取消 */
export interface AaSplitCancelledPayload {
  type: 'AA_SPLIT_CANCELLED';
  total_shares: number;
}

export interface OrderMovedPayload {
  type: 'ORDER_MOVED';
  source_table_id: string;
  source_table_name: string;
  target_table_id: string;
  target_table_name: string;
  target_zone_id?: string | null;
  target_zone_name?: string | null;
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
  payments: PaymentRecord[];
  paid_item_quantities: Record<string, number>;
  paid_amount: number;
  has_amount_split: boolean;
  aa_total_shares?: number | null;
  aa_paid_shares: number;
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

/** Order info updated (receipt_number is immutable - set at OpenTable) */
export interface OrderInfoUpdatedPayload {
  type: 'ORDER_INFO_UPDATED';
  guest_count?: number | null;
  table_name?: string | null;
  is_pre_payment?: boolean | null;
}

export interface RuleSkipToggledPayload {
  type: 'RULE_SKIP_TOGGLED';
  rule_id: string;
  rule_name: string;
  skipped: boolean;
}

/** 订单级手动折扣已应用 */
export interface OrderDiscountAppliedPayload {
  type: 'ORDER_DISCOUNT_APPLIED';
  discount_percent?: number | null;
  discount_fixed?: number | null;
  previous_discount_percent?: number | null;
  previous_discount_fixed?: number | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
  subtotal: number;
  discount: number;
  total: number;
}

/** 订单级附加费已应用 */
export interface OrderSurchargeAppliedPayload {
  type: 'ORDER_SURCHARGE_APPLIED';
  surcharge_percent?: number | null;
  surcharge_amount?: number | null;
  previous_surcharge_percent?: number | null;
  previous_surcharge_amount?: number | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
  subtotal: number;
  surcharge: number;
  total: number;
}

/** 订单备注已添加/更新 */
export interface OrderNoteAddedPayload {
  type: 'ORDER_NOTE_ADDED';
  /** 新备注内容 */
  note: string;
  /** 之前的备注（用于审计） */
  previous_note?: string | null;
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
  | AddItemsCommand
  | ModifyItemCommand
  | RemoveItemCommand
  | AddPaymentCommand
  | CancelPaymentCommand
  | SplitByItemsCommand
  | SplitByAmountCommand
  | StartAaSplitCommand
  | PayAaSplitCommand
  | MoveOrderCommand
  | MergeOrdersCommand
  | UpdateOrderInfoCommand
  | ToggleRuleSkipCommand
  | CompItemCommand
  | UncompItemCommand
  | ApplyOrderDiscountCommand
  | ApplyOrderSurchargeCommand
  | AddOrderNoteCommand;

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
  /** 服务类型（堂食/外卖，结单时确认，仅零售订单） */
  service_type?: ServiceType | null;
}

export interface VoidOrderCommand {
  type: 'VOID_ORDER';
  order_id: string;
  /** 作废类型（默认 CANCELLED） */
  void_type?: VoidType;
  /** 损失原因（仅 LOSS_SETTLED 时使用） */
  loss_reason?: LossReason | null;
  /** 损失金额（仅 LOSS_SETTLED 时使用） */
  loss_amount?: number | null;
  /** 备注 */
  note?: string | null;
  /** 授权人 ID（提权操作时传入） */
  authorizer_id?: string | null;
  /** 授权人名称 */
  authorizer_name?: string | null;
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
  affected_quantity?: number | null;
  changes: ItemChanges;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

export interface RemoveItemCommand {
  type: 'REMOVE_ITEM';
  order_id: string;
  instance_id: string;
  quantity?: number | null;
  reason?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

/** Comp (gift) an item - mark as free with reason and authorizer */
export interface CompItemCommand {
  type: 'COMP_ITEM';
  order_id: string;
  instance_id: string;
  /** Number of items to comp (can be partial) */
  quantity: number;
  /** Reason for comp (required for audit) */
  reason: string;
  /** Authorizer ID (required) */
  authorizer_id: string;
  /** Authorizer name (required) */
  authorizer_name: string;
}

/** Uncomp (reverse gift) an item - restore original price */
export interface UncompItemCommand {
  type: 'UNCOMP_ITEM';
  order_id: string;
  /** Instance ID of the comped item to uncomp */
  instance_id: string;
  /** Authorizer ID (required) */
  authorizer_id: string;
  /** Authorizer name (required) */
  authorizer_name: string;
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

/** 菜品分单 */
export interface SplitByItemsCommand {
  type: 'SPLIT_BY_ITEMS';
  order_id: string;
  payment_method: string;
  items: SplitItem[];
  tendered?: number | null;
}

/** 金额分单 */
export interface SplitByAmountCommand {
  type: 'SPLIT_BY_AMOUNT';
  order_id: string;
  split_amount: number;
  payment_method: string;
  tendered?: number | null;
}

/** AA 开始（锁定人数 + 支付第一份） */
export interface StartAaSplitCommand {
  type: 'START_AA_SPLIT';
  order_id: string;
  total_shares: number;
  shares: number;
  payment_method: string;
  tendered?: number | null;
}

/** AA 后续支付 */
export interface PayAaSplitCommand {
  type: 'PAY_AA_SPLIT';
  order_id: string;
  shares: number;
  payment_method: string;
  tendered?: number | null;
}

export interface MoveOrderCommand {
  type: 'MOVE_ORDER';
  order_id: string;
  target_table_id: string;
  target_table_name: string;
  target_zone_id?: string | null;
  target_zone_name?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

export interface MergeOrdersCommand {
  type: 'MERGE_ORDERS';
  source_order_id: string;
  target_order_id: string;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

/** Update order info (receipt_number is immutable - set at OpenTable) */
export interface UpdateOrderInfoCommand {
  type: 'UPDATE_ORDER_INFO';
  order_id: string;
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

/** 应用订单级手动折扣 */
export interface ApplyOrderDiscountCommand {
  type: 'APPLY_ORDER_DISCOUNT';
  order_id: string;
  /** 百分比折扣 (0-100)，null = 清除 */
  discount_percent?: number | null;
  /** 固定金额折扣，null = 清除 */
  discount_fixed?: number | null;
  reason?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

/** 应用订单级附加费 */
export interface ApplyOrderSurchargeCommand {
  type: 'APPLY_ORDER_SURCHARGE';
  order_id: string;
  /** 百分比附加费，与 surcharge_amount 互斥 */
  surcharge_percent?: number | null;
  /** 固定附加费金额，与 surcharge_percent 互斥；两者都为 null = 清除 */
  surcharge_amount?: number | null;
  reason?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
}

/** 添加/清除订单备注 */
export interface AddOrderNoteCommand {
  type: 'ADD_ORDER_NOTE';
  order_id: string;
  /** 备注内容，空字符串 = 清除备注 */
  note: string;
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
  /** 服务类型（堂食/外卖，零售订单使用） */
  service_type?: ServiceType | null;
  /** 叫号（服务器生成，零售订单使用） */
  queue_number?: number | null;
  status: OrderStatus;

  // === Void Information (only when status === 'VOID') ===
  /** Void type (CANCELLED or LOSS_SETTLED) */
  void_type?: VoidType;
  /** Loss reason (only for LOSS_SETTLED) */
  loss_reason?: LossReason;
  /** Loss amount (only for LOSS_SETTLED) */
  loss_amount?: number;
  /** Void note */
  void_note?: string;

  items: CartItemSnapshot[];
  /** Comp records (audit trail for comped items) */
  comps?: CompRecord[];
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
  /** Total discount amount (order-level) */
  discount: number;
  /** Comp total amount (赠送减免总额) */
  comp_total_amount: number;
  /** Order-level manual discount computed amount (整单手动折扣实际金额) */
  order_manual_discount_amount: number;
  /** Order-level manual surcharge computed amount (整单手动附加费实际金额) */
  order_manual_surcharge_amount: number;
  /** Total amount to pay */
  total: number;
  /** Amount already paid */
  paid_amount: number;
  /** Remaining amount to pay (total - paid_amount) */
  remaining_amount: number;
  /** Quantities paid per item (for split bill) */
  paid_item_quantities?: Record<string, number>;
  /** Whether this order has amount-based split payments (金额分单) */
  has_amount_split?: boolean;
  /** AA split: total shares (locked after first AA payment) */
  aa_total_shares?: number | null;
  /** AA split: number of shares already paid */
  aa_paid_shares?: number;
  /** Server-generated receipt number (always present from OpenTable) */
  receipt_number: string;
  is_pre_payment?: boolean;
  /** 订单备注 */
  note?: string | null;

  // === Order-level Rule Adjustments ===
  /** Order-level rule discount amount */
  order_rule_discount_amount?: number | null;
  /** Order-level rule surcharge amount */
  order_rule_surcharge_amount?: number | null;
  /** Order-level applied rules */
  order_applied_rules?: AppliedRule[] | null;

  // === Order-level Manual Adjustments ===
  /** Order-level manual discount percentage */
  order_manual_discount_percent?: number | null;
  /** Order-level manual discount fixed amount */
  order_manual_discount_fixed?: number | null;
  /** Order-level manual surcharge percentage */
  order_manual_surcharge_percent?: number | null;
  /** Order-level manual surcharge fixed amount */
  order_manual_surcharge_fixed?: number | null;

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

  // === Rule Adjustments ===
  /** Rule discount amount (calculated from price rules) */
  rule_discount_amount?: number | null;
  /** Rule surcharge amount (calculated from price rules) */
  rule_surcharge_amount?: number | null;
  /** Applied price rules list */
  applied_rules?: AppliedRule[] | null;

  // === Computed Fields ===
  /** Unit price for display (computed by backend: price with manual discount and rule adjustments) */
  unit_price?: number | null;
  /** Line total (computed by backend: unit_price * quantity) */
  line_total?: number | null;
  /** Tax amount for this item */
  tax?: number | null;
  /** Tax rate percentage (e.g., 21 for 21% IVA) */
  tax_rate?: number | null;

  note?: string | null;
  authorizer_id?: string | null;
  authorizer_name?: string | null;
  /** Category name snapshot (for statistics) */
  category_name?: string | null;
  /** Whether this item has been comped (gifted) */
  is_comped?: boolean;
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
  rule_type: 'DISCOUNT' | 'SURCHARGE';
  adjustment_type: 'PERCENTAGE' | 'FIXED_AMOUNT';
  product_scope: 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
  /** Zone scope: "zone:all", "zone:retail", or specific zone ID */
  zone_scope: string;
  adjustment_value: number;
  calculated_amount: number;
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
  /** Option quantity (default: 1) */
  quantity?: number;
}

export interface SpecificationInfo {
  id: string;
  name: string;
  receipt_name?: string | null;
  price?: number | null;
  /** Whether product has multiple specs (for display purposes) */
  is_multi_spec?: boolean;
}

export interface ItemChanges {
  price?: number | null;
  quantity?: number | null;
  /** Manual discount percentage (0-100) */
  manual_discount_percent?: number | null;
  note?: string | null;
  selected_options?: ItemOption[] | null;
  selected_specification?: SpecificationInfo | null;
}

export interface SplitItem {
  instance_id: string;
  name: string;
  quantity: number;
  unit_price: number;
}

export interface PaymentSummaryItem {
  method: string;
  amount: number;
}

/** Split type for categorizing split payments */
export type SplitType = 'ITEM_SPLIT' | 'AMOUNT_SPLIT' | 'AA_SPLIT';

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
  /** AA split: number of shares this payment covers (for rollback on cancel) */
  aa_shares?: number | null;
  /** Split type: which split mode produced this payment */
  split_type?: SplitType | null;
}

/**
 * Comp record (audit trail for comped items, matches Rust CompRecord)
 */
export interface CompRecord {
  comp_id: string;
  instance_id: string;
  source_instance_id: string;
  item_name: string;
  quantity: number;
  original_price: number;
  reason: string;
  authorizer_id: string;
  authorizer_name: string;
  timestamp: number;
}

// ============================================================================
// Connection State Type
// ============================================================================

export type OrderConnectionState = 'connected' | 'disconnected' | 'syncing';
