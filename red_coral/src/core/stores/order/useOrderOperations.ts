/**
 * Order Operations - Business logic for order operations
 *
 * These functions handle complex order workflows using the new event-sourcing architecture.
 * All write operations are async and go through the backend.
 */

import { invokeApi } from '@/infrastructure/api/tauri-client';
import { HeldOrder, CartItem, PaymentRecord, Table, Zone } from '@/core/domain/types';
import { Currency } from '@/utils/currency';
import { calculateDiscountAmount, calculateItemFinalPrice } from '@/utils/pricing';
import { useActiveOrdersStore } from './useActiveOrdersStore';
import { useReceiptStore } from './useReceiptStore';
import { useCheckoutStore } from './useCheckoutStore';
import { toHeldOrder } from './orderAdapter';
import { useBridgeStore } from '@/core/stores/bridge/useBridgeStore';
import type {
  OrderCommand,
  OrderCommandPayload,
  CommandResponse,
  CartItemInput,
  SurchargeConfig,
  PaymentMethod,
} from '@/core/domain/types/orderEvent';

// ============================================================================
// Helper Functions (Pure Logic)
// ============================================================================

const applySurchargeToItems = (
  items: CartItem[],
  surchargePercentage: number,
  surchargePerItem: number
): CartItem[] => {
  return items.map(item => {
    const originalPrice = item.originalPrice ?? 0;
    const discountAmount = calculateDiscountAmount(originalPrice, item.discountPercent || 0);
    const discountedBase = Currency.sub(originalPrice, discountAmount);

    let itemSurchargeAmount = Currency.toDecimal(0);
    if (surchargePercentage > 0) {
      itemSurchargeAmount = Currency.floor2((discountedBase.toNumber() * surchargePercentage) / 100);
    } else if (surchargePerItem > 0) {
      itemSurchargeAmount = Currency.floor2(surchargePerItem);
    }

    const itemForCalc = { ...item, surcharge: itemSurchargeAmount.toNumber() };
    const finalItemPrice = calculateItemFinalPrice(itemForCalc);

    return {
      ...item,
      price: finalItemPrice.toNumber(),
      surcharge: itemSurchargeAmount.toNumber()
    };
  });
};

const calculateSurchargeInfo = (existingOrder: HeldOrder | undefined, zone: Zone | undefined) => {
  let surchargePerItem = 0;
  let surchargePercentage = 0;
  let orderSurchargeInfo: SurchargeConfig | undefined;

  if (existingOrder?.surcharge) {
    if (existingOrder.surcharge.type === 'percentage') {
      surchargePercentage = existingOrder.surcharge.value || 0;
    } else {
      surchargePerItem = existingOrder.surcharge.value || 0;
    }
    orderSurchargeInfo = {
      type: existingOrder.surcharge.type,
      value: existingOrder.surcharge.value || 0,
      amount: existingOrder.surcharge.amount || 0,
      total: existingOrder.surcharge.total || 0,
      name: existingOrder.surcharge.name || null,
    };
  } else if (zone?.surcharge_type && zone?.surcharge_amount) {
    if (zone.surcharge_type === 'percentage') {
      surchargePercentage = zone.surcharge_amount;
      orderSurchargeInfo = {
        type: 'percentage',
        value: surchargePercentage,
        amount: 0,
        total: 0,
        name: zone.name || null,
      };
    } else {
      surchargePerItem = zone.surcharge_amount;
      orderSurchargeInfo = {
        type: 'fixed',
        value: surchargePerItem,
        amount: 0,
        total: 0,
        name: zone.name || null,
      };
    }
  }

  return { surchargePerItem, surchargePercentage, orderSurchargeInfo };
};

const prepareItemsWithSurcharge = (
  cart: CartItem[],
  surchargePercentage: number,
  surchargePerItem: number
) => {
  return applySurchargeToItems(cart, surchargePercentage, surchargePerItem);
};

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
 * Convert CartItem to CartItemInput for backend
 */
function toCartItemInput(item: CartItem): CartItemInput {
  // Convert ItemAttributeSelection[] to ItemOption[] if present
  const selectedOptions = item.selectedOptions?.map(opt => ({
    attribute_id: opt.attribute_id,
    attribute_name: opt.attribute_name ?? opt.name,
    option_idx: opt.option_idx,
    option_name: opt.value,
    price_modifier: opt.price_modifier ?? null,
  })) ?? null;

  return {
    product_id: item.productId ?? item.id,
    name: item.name,
    price: item.price,
    original_price: item.originalPrice ?? item.price,
    quantity: item.quantity,
    note: item.note ?? null,
    discount_percent: item.discountPercent ?? null,
    surcharge: item.surcharge ?? null,
    selected_options: selectedOptions,
    selected_specification: item.selectedSpecification ? {
      id: item.selectedSpecification.id,
      name: item.selectedSpecification.name,
      receipt_name: item.selectedSpecification.receipt_name ?? null,
      price: item.selectedSpecification.price ?? null,
    } : null,
    authorizer_id: item.authorizerId ?? null,
    authorizer_name: item.authorizerName ?? null,
  };
}

// ============================================================================
// Internal Action Handlers
// ============================================================================

const handleMergeToOrder = async (
  orderId: string,
  cart: CartItem[],
  surchargePercentage: number,
  surchargePerItem: number,
): Promise<'MERGED'> => {
  const itemsToAdd = prepareItemsWithSurcharge(cart, surchargePercentage, surchargePerItem);

  const command = createCommand({
    type: 'ADD_ITEMS',
    order_id: orderId,
    items: itemsToAdd.map(toCartItemInput),
  });

  await sendCommand(command);
  return 'MERGED';
};

const handleCreateNewOrder = async (
  tableId: string,
  table: Table,
  guestCount: number,
  zone: Zone | undefined,
  cart: CartItem[],
  surchargeInfo: { surchargePercentage: number; surchargePerItem: number; orderSurchargeInfo?: SurchargeConfig },
): Promise<'CREATED'> => {
  const receiptNumber = useReceiptStore.getState().generateReceiptNumber();
  const itemsToAdd = prepareItemsWithSurcharge(
    cart,
    surchargeInfo.surchargePercentage,
    surchargeInfo.surchargePerItem
  );

  // Create order
  const openCommand = createCommand({
    type: 'OPEN_TABLE',
    table_id: tableId,
    table_name: table.name,
    zone_id: zone?.id ? String(zone.id) : null,
    zone_name: zone?.name || null,
    guest_count: guestCount,
    is_retail: false,
    surcharge: surchargeInfo.orderSurchargeInfo || null,
  });

  const openResponse = await sendCommand(openCommand);
  if (!openResponse.success) {
    throw new Error(openResponse.error?.message || 'Failed to open table');
  }

  // Get the created order_id from response
  const orderId = openResponse.order_id || tableId;

  // Add items
  const addCommand = createCommand({
    type: 'ADD_ITEMS',
    order_id: orderId,
    items: itemsToAdd.map(toCartItemInput),
  });

  await sendCommand(addCommand);
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
  _enableIndividualMode?: boolean,
  _isIndividualMode?: boolean,
  zone?: Zone
): Promise<'MERGED' | 'CREATED' | 'RETRIEVED' | 'EMPTY'> => {
  const tableId = String(table.id);
  const store = useActiveOrdersStore.getState();
  const checkoutStore = useCheckoutStore.getState();

  const existingSnapshot = store.getOrderByTable(tableId);
  const existingOrder = existingSnapshot ? toHeldOrder(existingSnapshot) : undefined;

  // 1. If cart has items, we are ADDING (Merge) or CREATING
  if (cart.length > 0) {
    const surchargeInfo = calculateSurchargeInfo(existingOrder, zone);

    if (existingOrder && existingOrder.status === 'ACTIVE') {
      return handleMergeToOrder(
        existingSnapshot!.order_id,
        cart,
        surchargeInfo.surchargePercentage,
        surchargeInfo.surchargePerItem,
      );
    } else {
      return handleCreateNewOrder(
        tableId,
        table,
        guestCount,
        zone,
        cart,
        surchargeInfo,
      );
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

  const orderId = order.id || order.key || String(order.tableId || '');

  // Ensure receipt number
  let finalReceiptNumber = order.receiptNumber;
  if (!finalReceiptNumber || !finalReceiptNumber.startsWith('FAC')) {
    finalReceiptNumber = receiptStore.generateReceiptNumber();
  }

  // Add payments
  for (const payment of newPayments) {
    const paymentCommand = createCommand({
      type: 'ADD_PAYMENT',
      order_id: orderId,
      method: payment.method as PaymentMethod,
      amount: payment.amount,
      tendered: payment.tendered ?? null,
      note: payment.note ?? null,
    });
    await sendCommand(paymentCommand);
  }

  // Complete order
  const completeCommand = createCommand({
    type: 'COMPLETE_ORDER',
    order_id: orderId,
    receipt_number: finalReceiptNumber,
  });
  await sendCommand(completeCommand);

  // Cleanup payment session
  import('@/core/stores/order/usePaymentStore').then(({ usePaymentStore }) => {
    usePaymentStore.getState().clearSession(orderId);
  });

  // Return updated order (may take a moment for event to arrive)
  const snapshot = store.getOrder(orderId);
  return snapshot ? toHeldOrder(snapshot) : order;
};

/**
 * Void an order
 */
export const voidOrder = async (
  order: HeldOrder,
  reason?: string
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.id || order.key || String(order.tableId || '');

  const command = createCommand({
    type: 'VOID_ORDER',
    order_id: orderId,
    reason: reason || null,
  });
  await sendCommand(command);

  // Cleanup
  import('@/core/stores/order/usePaymentStore').then(({ usePaymentStore }) => {
    usePaymentStore.getState().clearSession(orderId);
  });

  const snapshot = store.getOrder(orderId);
  return snapshot ? toHeldOrder(snapshot) : order;
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

  const orderId = order.id || order.key || String(order.tableId || '');

  // Add payments
  for (const payment of newPayments) {
    const command = createCommand({
      type: 'ADD_PAYMENT',
      order_id: orderId,
      method: payment.method as PaymentMethod,
      amount: payment.amount,
      tendered: payment.tendered ?? null,
      note: payment.note ?? null,
    });
    await sendCommand(command);
  }

  // Sync checkout store
  const snapshot = store.getOrder(orderId);
  const updatedOrder = snapshot ? toHeldOrder(snapshot) : order;

  if (checkoutStore.checkoutOrder?.key === orderId) {
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
    items: { instanceId: string; name: string; quantity: number }[];
    paymentMethod: string;
    tendered?: number;
    change?: number;
  }
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.id || order.key || String(order.tableId || '');

  const command = createCommand({
    type: 'SPLIT_ORDER',
    order_id: orderId,
    split_amount: splitData.splitAmount,
    payment_method: splitData.paymentMethod,
    items: splitData.items.map(item => ({
      instance_id: item.instanceId,
      name: item.name,
      quantity: item.quantity,
    })),
  });

  await sendCommand(command);

  const snapshot = store.getOrder(orderId);
  return snapshot ? toHeldOrder(snapshot) : order;
};

/**
 * Update order info (receipt number, guest count, etc.)
 */
export const updateOrderInfo = async (
  order: HeldOrder,
  info: {
    receiptNumber?: string;
    guestCount?: number;
    tableName?: string;
    isPrePayment?: boolean;
  }
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.id || order.key || String(order.tableId || '');

  const command = createCommand({
    type: 'UPDATE_ORDER_INFO',
    order_id: orderId,
    receipt_number: info.receiptNumber ?? null,
    guest_count: info.guestCount ?? null,
    table_name: info.tableName ?? null,
    is_pre_payment: info.isPrePayment ?? null,
  });

  await sendCommand(command);

  const snapshot = store.getOrder(orderId);
  return snapshot ? toHeldOrder(snapshot) : order;
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
  const orderId = order.id || order.key || String(order.tableId || '');

  const command = createCommand({
    type: 'MOVE_ORDER',
    order_id: orderId,
    target_table_id: targetTableId,
    target_table_name: targetTableName,
    target_zone_name: targetZoneName ?? null,
  });

  await sendCommand(command);

  // Order will be at new table ID after move
  const snapshot = store.getOrder(targetTableId);
  return snapshot ? toHeldOrder(snapshot) : order;
};

/**
 * Merge source order into target order
 */
export const mergeOrders = async (
  sourceOrder: HeldOrder,
  targetOrder: HeldOrder
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const sourceId = sourceOrder.id || sourceOrder.key || String(sourceOrder.tableId || '');
  const targetId = targetOrder.id || targetOrder.key || String(targetOrder.tableId || '');

  const command = createCommand({
    type: 'MERGE_ORDERS',
    source_order_id: sourceId,
    target_order_id: targetId,
  });

  await sendCommand(command);

  const snapshot = store.getOrder(targetId);
  return snapshot ? toHeldOrder(snapshot) : targetOrder;
};

/**
 * Set surcharge exempt status
 */
export const setSurchargeExempt = async (
  order: HeldOrder,
  exempt: boolean
): Promise<HeldOrder> => {
  const store = useActiveOrdersStore.getState();
  const orderId = order.id || order.key || String(order.tableId || '');

  const command = createCommand({
    type: 'SET_SURCHARGE_EXEMPT',
    order_id: orderId,
    exempt,
  });

  await sendCommand(command);

  const snapshot = store.getOrder(orderId);
  return snapshot ? toHeldOrder(snapshot) : order;
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

  await sendCommand(command);
};

/**
 * Modify an item in an order
 */
export const modifyItem = async (
  orderId: string,
  instanceId: string,
  changes: {
    price?: number;
    quantity?: number;
    discountPercent?: number;
    surcharge?: number;
    note?: string;
  }
): Promise<void> => {
  const command = createCommand({
    type: 'MODIFY_ITEM',
    order_id: orderId,
    instance_id: instanceId,
    changes: {
      price: changes.price ?? null,
      quantity: changes.quantity ?? null,
      discount_percent: changes.discountPercent ?? null,
      surcharge: changes.surcharge ?? null,
      note: changes.note ?? null,
    },
  });

  await sendCommand(command);
};

/**
 * Remove an item from an order (soft delete)
 */
export const removeItem = async (
  orderId: string,
  instanceId: string,
  reason?: string,
  quantity?: number
): Promise<void> => {
  const command = createCommand({
    type: 'REMOVE_ITEM',
    order_id: orderId,
    instance_id: instanceId,
    reason: reason ?? null,
    quantity: quantity ?? null,
  });

  await sendCommand(command);
};
