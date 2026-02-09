/**
 * Order adjustment commands: discount, surcharge, note, rule skip, move, merge, update info.
 */

import { createCommand } from '../commandUtils';
import { sendCommand, ensureSuccess } from './sendCommand';

/**
 * Apply order-level manual discount (percent or fixed, mutually exclusive).
 * Both null = clear discount.
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
 * Apply order-level surcharge (fixed amount).
 * null = clear surcharge.
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

/**
 * Add or clear order-level note.
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

/**
 * Toggle rule skip status for an order.
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

/**
 * Move order to a different table.
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
 * Merge source order into target order.
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

/**
 * Update order info (guest count, table name, etc.).
 * Note: receipt_number is immutable (set at OpenTable).
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
