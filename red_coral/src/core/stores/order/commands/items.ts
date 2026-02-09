/**
 * Item-level order commands: add, modify, remove, comp, uncomp.
 */

import { CartItem } from '@/core/domain/types';
import { createCommand } from '../commandUtils';
import { sendCommand, ensureSuccess } from './sendCommand';
import type {
  CartItemInput,
  ItemOption,
  SpecificationInfo,
} from '@/core/domain/types/orderEvent';

/**
 * Convert CartItem to CartItemInput for backend.
 */
export function toCartItemInput(item: CartItem): CartItemInput {
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
    original_price: item.original_price || item.price,
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

/**
 * Add items to an existing order.
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
 * Modify an item in an order.
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
 * Remove an item from an order (soft delete).
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
 * Comp (gift) an item — splits quantity and marks as free.
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
 * Uncomp (undo gift) an item — restore original price.
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
