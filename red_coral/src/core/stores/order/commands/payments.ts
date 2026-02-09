/**
 * Payment-related order commands: partial settle, cancel, split, AA split.
 */

import { PaymentRecord } from '@/core/domain/types';
import { createCommand } from '../commandUtils';
import { sendCommand, ensureSuccess } from './sendCommand';

/**
 * Partial settle (add payments without completing).
 * Fire & forget — UI updates via WebSocket event.
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
 * Cancel a payment.
 */
export const cancelPayment = async (
  orderId: string,
  paymentId: string,
  reason?: string,
  authorizer?: { id: number; name: string },
): Promise<void> => {
  const command = createCommand({
    type: 'CANCEL_PAYMENT',
    order_id: orderId,
    payment_id: paymentId,
    reason: reason ?? null,
    authorizer_id: authorizer?.id ?? null,
    authorizer_name: authorizer?.name ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Cancel payment');
};

/**
 * Split by items.
 * Fire & forget — UI updates via WebSocket event.
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
 * Split by amount.
 * Fire & forget — UI updates via WebSocket event.
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
 * Start AA split (lock headcount + pay first portion).
 * Fire & forget — UI updates via WebSocket event.
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
 * Pay subsequent AA split portion.
 * Fire & forget — UI updates via WebSocket event.
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
