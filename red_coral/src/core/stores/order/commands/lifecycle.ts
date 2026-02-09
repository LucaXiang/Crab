/**
 * Order lifecycle commands: create, table-select, complete, void.
 */

import { CartItem, Table, Zone } from '@/core/domain/types';
import { useActiveOrdersStore } from '../useActiveOrdersStore';
import { useCheckoutStore } from '../useCheckoutStore';
import { createCommand } from '../commandUtils';
import { sendCommand, ensureSuccess } from './sendCommand';
import { toCartItemInput } from './items';
import type { VoidOrderOptions } from './types';
import type { PaymentRecord } from '@/core/domain/types';
import type { ServiceType } from '@/core/domain/types/orderEvent';

// ============================================================================
// Internal helpers
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

  const orderId = openResponse.order_id;
  if (!orderId) {
    throw new Error('OPEN_TABLE command succeeded but no order_id returned');
  }

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
// Exported operations
// ============================================================================

/**
 * Create a retail order with cart items.
 * Returns the new order_id directly (needed for navigation).
 */
export const createRetailOrder = async (
  cart: CartItem[],
): Promise<string> => {
  if (cart.length === 0) {
    throw new Error('Cannot create retail order with empty cart');
  }

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
 * Handle table selection — creates new order or merges to existing.
 */
export const handleTableSelect = async (
  table: Table,
  guestCount: number,
  cart: CartItem[],
  zone?: Zone
): Promise<'MERGED' | 'CREATED' | 'RETRIEVED' | 'EMPTY'> => {
  const tableId = table.id;
  const store = useActiveOrdersStore.getState();
  const checkoutStore = useCheckoutStore.getState();

  const existingSnapshot = store.getOrderByTable(tableId);
  const existingOrder = existingSnapshot ? existingSnapshot : undefined;

  if (cart.length > 0) {
    if (existingOrder && existingOrder.status === 'ACTIVE') {
      return handleMergeToOrder(existingSnapshot!.order_id, cart);
    } else {
      return handleCreateNewOrder(tableId, table, guestCount, zone, cart);
    }
  }

  if (existingOrder) {
    checkoutStore.setCheckoutOrder(existingOrder);
    return 'RETRIEVED';
  }

  return 'EMPTY';
};

/**
 * Complete an order with payments.
 * Fire & forget — UI updates via WebSocket event.
 */
export const completeOrder = async (
  orderId: string,
  newPayments: PaymentRecord[],
  serviceType?: ServiceType | null,
): Promise<void> => {
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

  const completeCommand = createCommand({
    type: 'COMPLETE_ORDER',
    order_id: orderId,
    service_type: serviceType ?? null,
  });
  const completeResponse = await sendCommand(completeCommand);
  ensureSuccess(completeResponse, 'Complete order');
};

/**
 * Void an order.
 * Fire & forget — UI updates via WebSocket event.
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
