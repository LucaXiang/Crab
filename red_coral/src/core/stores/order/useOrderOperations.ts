/**
 * Order Operations - Business logic for order operations
 *
 * These functions handle complex order workflows using the new event-sourcing architecture.
 * All write operations are async and go through the backend.
 *
 * **Server Authority Model:**
 * - Client only sends commands, never expects data back
 * - UI updates reactively through WebSocket events -> Store -> React re-render
 * - Multi-terminal: all clients subscribe to same event stream
 */

import { invokeApi } from '@/infrastructure/api/tauri-client';
import { CartItem, PaymentRecord, Table, Zone } from '@/core/domain/types';
import { useActiveOrdersStore } from './useActiveOrdersStore';
import { useCheckoutStore } from './useCheckoutStore';
import { createCommand } from './commandUtils';
import type {
  OrderCommand,
  CommandResponse,
  CartItemInput,
  ServiceType,
  ItemOption,
  SpecificationInfo,
} from '@/core/domain/types/orderEvent';

async function sendCommand(command: OrderCommand): Promise<CommandResponse> {
  try {
    return await invokeApi<CommandResponse>('order_execute_command', { command });
  } catch (error: unknown) {
    console.error('Command failed:', error);
    return {
      command_id: command.command_id,
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: error instanceof Error ? error.message : 'Command execution failed',
      },
    };
  }
}

/**
 * Helper to ensure command succeeded, throws on failure
 */
function ensureSuccess(response: CommandResponse, context: string): void {
  if (!response.success) {
    const message = response.error?.message || `${context} failed`;
    console.error(`[OrderOps] ${context}:`, message);
    throw new Error(message);
  }
}

/**
 * Convert CartItem to CartItemInput for backend
 */
function toCartItemInput(item: CartItem): CartItemInput {
  // CartItem.selected_options is already ItemOption[], pass through directly
  const selectedOptions = item.selected_options?.map(opt => ({
    attribute_id: opt.attribute_id,
    attribute_name: opt.attribute_name,
    option_idx: opt.option_idx,
    option_name: opt.option_name,
    price_modifier: opt.price_modifier ?? null,
    quantity: opt.quantity ?? 1,
  })) ?? null;

  return {
    product_id: item.id,
    name: item.name,
    price: item.price,
    original_price: item.original_price ?? item.price,
    quantity: item.quantity,
    note: item.note ?? null,
    manual_discount_percent: item.manual_discount_percent ?? null,
    selected_options: selectedOptions,
    selected_specification: item.selected_specification ? {
      id: item.selected_specification.id,
      name: item.selected_specification.name,
      receipt_name: item.selected_specification.receipt_name ?? null,
      price: item.selected_specification.price ?? null,
    } : null,
    authorizer_id: item.authorizer_id ?? null,
    authorizer_name: item.authorizer_name ?? null,
  };
}

// ============================================================================
// Internal Action Handlers
// ============================================================================

const handleMergeToOrder = async (
  orderId: string,
  cart: CartItem[],
): Promise<'MERGED'> => {
  const command = createCommand({
    type: 'ADD_ITEMS',
    order_id: orderId,
    items: cart.map(toCartItemInput),
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Add items to order');
  return 'MERGED';
};

const handleCreateNewOrder = async (
  tableId: number,
  table: Table,
  guestCount: number,
  zone: Zone | undefined,
  cart: CartItem[],
): Promise<'CREATED'> => {
  // Create order
  const openCommand = createCommand({
    type: 'OPEN_TABLE',
    table_id: tableId,
    table_name: table.name,
    zone_id: zone?.id ?? null,
    zone_name: zone?.name || null,
    guest_count: guestCount,
    is_retail: false,
  });

  const openResponse = await sendCommand(openCommand);
  ensureSuccess(openResponse, 'Open table');

  // Get the created order_id from response (required for OPEN_TABLE success)
  const orderId = openResponse.order_id;
  if (!orderId) {
    throw new Error('OPEN_TABLE command succeeded but no order_id returned');
  }

  // Add items (Price Rules will be applied by backend)
  const addCommand = createCommand({
    type: 'ADD_ITEMS',
    order_id: orderId,
    items: cart.map(toCartItemInput),
  });

  const addResponse = await sendCommand(addCommand);
  ensureSuccess(addResponse, 'Add items to order');
  return 'CREATED';
};

// ============================================================================
// Exported Operations (Async) - Fire & Forget Commands
// ============================================================================

/**
 * Create a retail order with cart items
 * Returns the new order_id directly (needed for navigation)
 */
export const createRetailOrder = async (
  cart: CartItem[],
): Promise<string> => {
  if (cart.length === 0) {
    throw new Error('Cannot create retail order with empty cart');
  }

  // 1. Create retail order (no table_id, is_retail = true)
  // service_type 在结单时设置，不在开台时传入
  const openCommand = createCommand({
    type: 'OPEN_TABLE',
    table_id: null,
    table_name: null,
    zone_id: null,
    zone_name: null,
    guest_count: 1,
    is_retail: true,
  });

  const openResponse = await sendCommand(openCommand);
  ensureSuccess(openResponse, 'Create retail order');

  const orderId = openResponse.order_id;
  if (!orderId) {
    throw new Error('Create retail order succeeded but no order_id returned');
  }

  // 2. Add items to the order
  const addCommand = createCommand({
    type: 'ADD_ITEMS',
    order_id: orderId,
    items: cart.map(toCartItemInput),
  });

  const addResponse = await sendCommand(addCommand);
  ensureSuccess(addResponse, 'Add items to retail order');

  return orderId;
};

/**
 * Handle table selection - creates new order or merges to existing
 */
export const handleTableSelect = async (
  table: Table,
  guestCount: number,
  cart: CartItem[],
  _totalAmount: number,
  zone?: Zone
): Promise<'MERGED' | 'CREATED' | 'RETRIEVED' | 'EMPTY'> => {
  const tableId = table.id;
  const store = useActiveOrdersStore.getState();
  const checkoutStore = useCheckoutStore.getState();

  const existingSnapshot = store.getOrderByTable(tableId);
  const existingOrder = existingSnapshot ? existingSnapshot : undefined;

  // 1. If cart has items, we are ADDING (Merge) or CREATING
  if (cart.length > 0) {
    if (existingOrder && existingOrder.status === 'ACTIVE') {
      return handleMergeToOrder(existingSnapshot!.order_id, cart);
    } else {
      return handleCreateNewOrder(tableId, table, guestCount, zone, cart);
    }
  }

  // 2. RETRIEVE Logic (No items in cart)
  if (existingOrder) {
    checkoutStore.setCheckoutOrder(existingOrder);
    return 'RETRIEVED';
  }

  return 'EMPTY';
};

/**
 * Complete an order with payments
 * Fire & forget - UI updates via WebSocket event
 * Note: receipt_number is server-generated at OpenTable, no need to pass
 */
export const completeOrder = async (
  orderId: string,
  newPayments: PaymentRecord[],
  serviceType?: ServiceType | null,
): Promise<void> => {
  // Add payments
  for (const payment of newPayments) {
    const paymentCommand = createCommand({
      type: 'ADD_PAYMENT',
      order_id: orderId,
      payment: {
        method: payment.method,
        amount: payment.amount,
        tendered: payment.tendered ?? null,
        note: payment.note ?? null,
      },
    });
    const paymentResponse = await sendCommand(paymentCommand);
    ensureSuccess(paymentResponse, 'Add payment');
  }

  // Complete order (server uses snapshot's receipt_number)
  const completeCommand = createCommand({
    type: 'COMPLETE_ORDER',
    order_id: orderId,
    service_type: serviceType ?? null,
  });
  const completeResponse = await sendCommand(completeCommand);
  ensureSuccess(completeResponse, 'Complete order');
};

/** 作废订单选项 */
export interface VoidOrderOptions {
  /** 作废类型（默认 CANCELLED） */
  voidType?: 'CANCELLED' | 'LOSS_SETTLED';
  /** 损失原因（仅 LOSS_SETTLED 时使用） */
  lossReason?: 'CUSTOMER_FLED' | 'CUSTOMER_INSOLVENT' | 'OTHER';
  /** 损失金额（仅 LOSS_SETTLED 时使用） */
  lossAmount?: number;
  /** 备注 */
  note?: string;
  /** 授权人 ID（提权操作时传入） */
  authorizerId?: number | null;
  /** 授权人名称 */
  authorizerName?: string | null;
}

/**
 * Void an order
 * Fire & forget - UI updates via WebSocket event
 */
export const voidOrder = async (
  orderId: string,
  options?: VoidOrderOptions
): Promise<void> => {
  const command = createCommand({
    type: 'VOID_ORDER',
    order_id: orderId,
    void_type: options?.voidType ?? 'CANCELLED',
    loss_reason: options?.lossReason ?? null,
    loss_amount: options?.lossAmount ?? null,
    note: options?.note ?? null,
    authorizer_id: options?.authorizerId ?? null,
    authorizer_name: options?.authorizerName ?? null,
  });
  const response = await sendCommand(command);
  ensureSuccess(response, 'Void order');
};

/**
 * Partial settle (add payments without completing)
 * Fire & forget - UI updates via WebSocket event
 */
export const partialSettle = async (
  orderId: string,
  newPayments: PaymentRecord[],
): Promise<void> => {
  for (const payment of newPayments) {
    const command = createCommand({
      type: 'ADD_PAYMENT',
      order_id: orderId,
      payment: {
        method: payment.method,
        amount: payment.amount,
        tendered: payment.tendered ?? null,
        note: payment.note ?? null,
      },
    });
    const response = await sendCommand(command);
    ensureSuccess(response, 'Add payment');
  }
};


/**
 * Split by items (菜品分单)
 * Fire & forget - UI updates via WebSocket event
 */
export const splitByItems = async (
  orderId: string,
  items: { instance_id: string; name: string; quantity: number; unit_price: number }[],
  paymentMethod: string,
  tendered?: number,
): Promise<void> => {
  const command = createCommand({
    type: 'SPLIT_BY_ITEMS',
    order_id: orderId,
    payment_method: paymentMethod,
    items: items.map(item => ({
      instance_id: item.instance_id,
      name: item.name,
      quantity: item.quantity,
      unit_price: item.unit_price,
    })),
    tendered: tendered ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Split by items');
};

/**
 * Split by amount (金额分单)
 * Fire & forget - UI updates via WebSocket event
 */
export const splitByAmount = async (
  orderId: string,
  splitAmount: number,
  paymentMethod: string,
  tendered?: number,
): Promise<void> => {
  const command = createCommand({
    type: 'SPLIT_BY_AMOUNT',
    order_id: orderId,
    split_amount: splitAmount,
    payment_method: paymentMethod,
    tendered: tendered ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Split by amount');
};

/**
 * Start AA split (锁定人数 + 支付第一份)
 * Fire & forget - UI updates via WebSocket event
 */
export const startAaSplit = async (
  orderId: string,
  totalShares: number,
  shares: number,
  paymentMethod: string,
  tendered?: number,
): Promise<void> => {
  const command = createCommand({
    type: 'START_AA_SPLIT',
    order_id: orderId,
    total_shares: totalShares,
    shares,
    payment_method: paymentMethod,
    tendered: tendered ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Start AA split');
};

/**
 * Pay AA split (后续 AA 支付)
 * Fire & forget - UI updates via WebSocket event
 */
export const payAaSplit = async (
  orderId: string,
  shares: number,
  paymentMethod: string,
  tendered?: number,
): Promise<void> => {
  const command = createCommand({
    type: 'PAY_AA_SPLIT',
    order_id: orderId,
    shares,
    payment_method: paymentMethod,
    tendered: tendered ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Pay AA split');
};

/**
 * Update order info (guest count, table name, etc.)
 * Fire & forget - UI updates via WebSocket event
 * Note: receipt_number is immutable (set at OpenTable)
 */
export const updateOrderInfo = async (
  orderId: string,
  info: {
    guest_count?: number;
    table_name?: string;
    is_pre_payment?: boolean;
  }
): Promise<void> => {
  const command = createCommand({
    type: 'UPDATE_ORDER_INFO',
    order_id: orderId,
    guest_count: info.guest_count ?? null,
    table_name: info.table_name ?? null,
    is_pre_payment: info.is_pre_payment ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Update order info');
};

/**
 * Move order to a different table
 * Fire & forget - UI updates via WebSocket event
 */
export const moveOrder = async (
  orderId: string,
  targetTableId: number,
  targetTableName: string,
  targetZoneId?: number | null,
  targetZoneName?: string | null,
  authorizer?: { id: number; name: string },
): Promise<void> => {
  const command = createCommand({
    type: 'MOVE_ORDER',
    order_id: orderId,
    target_table_id: targetTableId,
    target_table_name: targetTableName,
    target_zone_id: targetZoneId ?? null,
    target_zone_name: targetZoneName ?? null,
    authorizer_id: authorizer?.id ?? null,
    authorizer_name: authorizer?.name ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Move order');
};

/**
 * Merge source order into target order
 * Fire & forget - UI updates via WebSocket event
 */
export const mergeOrders = async (
  sourceOrderId: string,
  targetOrderId: string,
  authorizer?: { id: number; name: string },
): Promise<void> => {
  const command = createCommand({
    type: 'MERGE_ORDERS',
    source_order_id: sourceOrderId,
    target_order_id: targetOrderId,
    authorizer_id: authorizer?.id ?? null,
    authorizer_name: authorizer?.name ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Merge orders');
};

// ============================================================================
// Item-Level Operations
// ============================================================================

/**
 * Add items to an existing order
 */
export const addItems = async (
  orderId: string,
  items: CartItem[]
): Promise<void> => {
  const command = createCommand({
    type: 'ADD_ITEMS',
    order_id: orderId,
    items: items.map(toCartItemInput),
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Add items');
};

/**
 * Modify an item in an order
 */
export const modifyItem = async (
  orderId: string,
  instance_id: string,
  changes: {
    price?: number;
    quantity?: number;
    manual_discount_percent?: number;
    note?: string;
    selected_options?: ItemOption[];
    selected_specification?: SpecificationInfo;
  },
  authorizer?: { id: number; name: string },
): Promise<void> => {
  const command = createCommand({
    type: 'MODIFY_ITEM',
    order_id: orderId,
    instance_id: instance_id,
    changes: {
      price: changes.price ?? null,
      quantity: changes.quantity ?? null,
      manual_discount_percent: changes.manual_discount_percent ?? null,
      note: changes.note ?? null,
      selected_options: changes.selected_options ?? null,
      selected_specification: changes.selected_specification ?? null,
    },
    authorizer_id: authorizer?.id ?? null,
    authorizer_name: authorizer?.name ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Modify item');
};

/**
 * Remove an item from an order (soft delete)
 */
export const removeItem = async (
  orderId: string,
  instance_id: string,
  reason?: string,
  quantity?: number,
  authorizer?: { id: number; name: string },
): Promise<void> => {
  const command = createCommand({
    type: 'REMOVE_ITEM',
    order_id: orderId,
    instance_id: instance_id,
    reason: reason ?? null,
    quantity: quantity ?? null,
    authorizer_id: authorizer?.id ?? null,
    authorizer_name: authorizer?.name ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Remove item');
};

/**
 * Toggle rule skip status for an order
 * @param orderId Order ID
 * @param ruleId Rule ID to toggle
 * @param skipped Whether to skip (true) or apply (false) the rule
 */
export const toggleRuleSkip = async (
  orderId: string,
  ruleId: number,
  skipped: boolean
): Promise<void> => {
  const command = createCommand({
    type: 'TOGGLE_RULE_SKIP',
    order_id: orderId,
    rule_id: ruleId,
    skipped,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Toggle rule skip');
};

// ============================================================================
// Comp Operations
// ============================================================================

/**
 * Comp (赠送) an item - splits quantity and marks as free
 * Fire & forget - UI updates via WebSocket event
 */
export const compItem = async (
  orderId: string,
  instanceId: string,
  quantity: number,
  reason: string,
  authorizer: { id: number; name: string },
): Promise<void> => {
  const command = createCommand({
    type: 'COMP_ITEM',
    order_id: orderId,
    instance_id: instanceId,
    quantity,
    reason,
    authorizer_id: authorizer.id,
    authorizer_name: authorizer.name,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Comp item');
};

/**
 * Uncomp (撤销赠送) an item - restore original price
 * Fire & forget - UI updates via WebSocket event
 */
export const uncompItem = async (
  orderId: string,
  instanceId: string,
  authorizer: { id: number; name: string },
): Promise<void> => {
  const command = createCommand({
    type: 'UNCOMP_ITEM',
    order_id: orderId,
    instance_id: instanceId,
    authorizer_id: authorizer.id,
    authorizer_name: authorizer.name,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Uncomp item');
};

// ============================================================================
// Order Note
// ============================================================================

/**
 * Add or clear order-level note (空字符串 = 清除备注)
 * Fire & forget - UI updates via WebSocket event
 */
export const addOrderNote = async (
  orderId: string,
  note: string,
): Promise<void> => {
  const command = createCommand({
    type: 'ADD_ORDER_NOTE',
    order_id: orderId,
    note,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Add order note');
};

// ============================================================================
// Order-level Adjustments
// ============================================================================

/**
 * Apply order-level manual discount (percent or fixed, mutually exclusive)
 * Both null = clear discount
 * Fire & forget - UI updates via WebSocket event
 */
export const applyOrderDiscount = async (
  orderId: string,
  options?: {
    discountPercent?: number;
    discountFixed?: number;
    authorizer?: { id: number; name: string };
  },
): Promise<void> => {
  const command = createCommand({
    type: 'APPLY_ORDER_DISCOUNT',
    order_id: orderId,
    discount_percent: options?.discountPercent ?? null,
    discount_fixed: options?.discountFixed ?? null,
    authorizer_id: options?.authorizer?.id ?? null,
    authorizer_name: options?.authorizer?.name ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Apply order discount');
};

/**
 * Apply order-level surcharge (fixed amount)
 * null = clear surcharge
 * Fire & forget - UI updates via WebSocket event
 */
export const applyOrderSurcharge = async (
  orderId: string,
  options?: {
    surchargePercent?: number;
    surchargeAmount?: number;
    authorizer?: { id: number; name: string };
  },
): Promise<void> => {
  const command = createCommand({
    type: 'APPLY_ORDER_SURCHARGE',
    order_id: orderId,
    surcharge_percent: options?.surchargePercent ?? null,
    surcharge_amount: options?.surchargeAmount ?? null,
    authorizer_id: options?.authorizer?.id ?? null,
    authorizer_name: options?.authorizer?.name ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Apply order surcharge');
};
