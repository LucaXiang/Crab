/**
 * Order Operations - Business logic for order operations
 *
 * These functions handle complex order workflows using the new event-sourcing architecture.
 * All write operations are async and go through the backend.
 */

import { invokeApi } from '@/infrastructure/api/tauri-client';
import { HeldOrder, CartItem, PaymentRecord, Table, Zone } from '@/core/domain/types';
import { useActiveOrdersStore } from './useActiveOrdersStore';
import { useReceiptStore } from './useReceiptStore';
import { useCheckoutStore } from './useCheckoutStore';
import { useBridgeStore } from '@/core/stores/bridge/useBridgeStore';
import type {
  OrderCommand,
  OrderCommandPayload,
  CommandResponse,
  CartItemInput,
} from '@/core/domain/types/orderEvent';

// ============================================================================
// Command Helpers
// ============================================================================

function generateCommandId(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

function createCommand(payload: OrderCommandPayload): OrderCommand {
  const session = useBridgeStore.getState().currentSession;
  const operatorId = session?.user_info?.id ?? 'unknown';
  const operatorName = session?.user_info?.username ?? 'Unknown';

  return {
    command_id: generateCommandId(),
    timestamp: Date.now(),
    operator_id: operatorId,
    operator_name: operatorName,
    payload,
  };
}

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
  })) ?? null;

  return {
    product_id: item.product_id ?? item.id,
    name: item.name,
    price: item.price,
    original_price: item.original_price ?? item.price,
    quantity: item.quantity,
    note: item.note ?? null,
    manual_discount_percent: item.manual_discount_percent ?? null,
    surcharge: item.surcharge ?? null,
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
  tableId: string,
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
    zone_id: zone?.id ? String(zone.id) : null,
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
// Exported Operations (Async)
// ============================================================================

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
  const tableId = String(table.id);
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
 */
export const completeOrder = async (
  order: HeldOrder,
  newPayments: PaymentRecord[],
): Promise<HeldOrder> => {
  const receiptStore = useReceiptStore.getState();
  const store = useActiveOrdersStore.getState();

  const orderId = order.order_id;

  // Ensure receipt number
  let finalReceiptNumber = order.receipt_number;
  if (!finalReceiptNumber || !finalReceiptNumber.startsWith('FAC')) {
    finalReceiptNumber = receiptStore.generateReceiptNumber();
  }

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

  // Complete order
  const completeCommand = createCommand({
    type: 'COMPLETE_ORDER',
    order_id: orderId,
    receipt_number: finalReceiptNumber,
  });
  const completeResponse = await sendCommand(completeCommand);
  ensureSuccess(completeResponse, 'Complete order');

  // Return updated order (may take a moment for event to arrive)
  const snapshot = store.getOrder(orderId);
  return snapshot ? snapshot : order;
};

/**
 * Void an order
 */
export const voidOrder = async (
  order: HeldOrder,
  reason?: string
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.order_id;

  const command = createCommand({
    type: 'VOID_ORDER',
    order_id: orderId,
    reason: reason || null,
  });
  const response = await sendCommand(command);
  ensureSuccess(response, 'Void order');

  const snapshot = store.getOrder(orderId);
  return snapshot ? snapshot : order;
};

/**
 * Partial settle (add payments without completing)
 */
export const partialSettle = async (
  order: HeldOrder,
  newPayments: PaymentRecord[],
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const checkoutStore = useCheckoutStore.getState();

  const orderId = order.order_id;

  // Add payments
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

  // Sync checkout store
  const snapshot = store.getOrder(orderId);
  const updatedOrder = snapshot ? snapshot : order;

  if (checkoutStore.checkoutOrder?.order_id === orderId) {
    checkoutStore.setCheckoutOrder(updatedOrder);
  }

  return updatedOrder;
};

/**
 * Ensure order exists in store (no-op with event sourcing - orders come from backend)
 * Kept for API compatibility
 */
export const ensureActiveOrder = (_order: HeldOrder): void => {
  // With event sourcing architecture, orders are managed by the backend
  // and synced via events. This function is a no-op for compatibility.
};

/**
 * Split order - process a split payment for specific items
 */
export const splitOrder = async (
  order: HeldOrder,
  splitData: {
    splitAmount: number;
    items: { instance_id: string; name: string; quantity: number }[];
    paymentMethod: string;
    tendered?: number;
    change?: number;
  }
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.order_id;

  const command = createCommand({
    type: 'SPLIT_ORDER',
    order_id: orderId,
    split_amount: splitData.splitAmount,
    payment_method: splitData.paymentMethod,
    items: splitData.items.map(item => ({
      instance_id: item.instance_id,
      name: item.name,
      quantity: item.quantity,
    })),
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Split order');

  const snapshot = store.getOrder(orderId);
  return snapshot ? snapshot : order;
};

/**
 * Update order info (receipt number, guest count, etc.)
 */
export const updateOrderInfo = async (
  order: HeldOrder,
  info: {
    receipt_number?: string;
    guest_count?: number;
    table_name?: string;
    is_pre_payment?: boolean;
  }
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.order_id;

  const command = createCommand({
    type: 'UPDATE_ORDER_INFO',
    order_id: orderId,
    receipt_number: info.receipt_number ?? null,
    guest_count: info.guest_count ?? null,
    table_name: info.table_name ?? null,
    is_pre_payment: info.is_pre_payment ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Update order info');

  const snapshot = store.getOrder(orderId);
  return snapshot ? snapshot : order;
};

/**
 * Move order to a different table
 */
export const moveOrder = async (
  order: HeldOrder,
  targetTableId: string,
  targetTableName: string,
  targetZoneName?: string
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.order_id;

  const command = createCommand({
    type: 'MOVE_ORDER',
    order_id: orderId,
    target_table_id: targetTableId,
    target_table_name: targetTableName,
    target_zone_name: targetZoneName ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Move order');

  // Order will be at new table ID after move
  const snapshot = store.getOrder(targetTableId);
  return snapshot ? snapshot : order;
};

/**
 * Merge source order into target order
 */
export const mergeOrders = async (
  sourceOrder: HeldOrder,
  targetOrder: HeldOrder
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const sourceId = sourceOrder.order_id;
  const targetId = targetOrder.order_id;

  const command = createCommand({
    type: 'MERGE_ORDERS',
    source_order_id: sourceId,
    target_order_id: targetId,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Merge orders');

  const snapshot = store.getOrder(targetId);
  return snapshot ? snapshot : targetOrder;
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
    surcharge?: number;
    note?: string;
  }
): Promise<void> => {
  const command = createCommand({
    type: 'MODIFY_ITEM',
    order_id: orderId,
    instance_id: instance_id,
    changes: {
      price: changes.price ?? null,
      quantity: changes.quantity ?? null,
      manual_discount_percent: changes.manual_discount_percent ?? null,
      surcharge: changes.surcharge ?? null,
      note: changes.note ?? null,
    },
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
  quantity?: number
): Promise<void> => {
  const command = createCommand({
    type: 'REMOVE_ITEM',
    order_id: orderId,
    instance_id: instance_id,
    reason: reason ?? null,
    quantity: quantity ?? null,
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
  ruleId: string,
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
