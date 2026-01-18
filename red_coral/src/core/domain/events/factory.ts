/**
 * Event Factory - Type-safe Event Creation
 *
 * Provides a clean, type-safe API for creating order events.
 */

import { CartItem, PaymentRecord, ItemAttributeSelection } from '@/core/domain/types';
import {
  OrderEventType,
  OrderEvent,
  TableOpenedEvent,
  ItemsAddedEvent,
  ItemModifiedEvent,
  ItemRemovedEvent,
  ItemRestoredEvent,
  PaymentAddedEvent,
  PaymentCancelledEvent,
  OrderCompletedEvent,
  OrderVoidedEvent,
  OrderRestoredEvent,
  OrderSurchargeExemptSetEvent,
  OrderMergedEvent,
  OrderMovedEvent,
  OrderMovedOutEvent,
  OrderMergedOutEvent,
  TableReassignedEvent,
  OrderInfoUpdatedEvent,
  OrderSplitEvent,
} from './types';

// ============ Base Factory ============

function createEvent<T extends OrderEvent>(
  type: T['type'],
  data: T['data'],
  options?: { userId?: string; note?: string; title?: string; summary?: string }
): T {
  return {
    id: `evt-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    type,
    timestamp: Date.now(),
    userId: options?.userId,
    note: options?.note,
    title: options?.title,
    summary: options?.summary,
    data,
  } as T;
}

// ============ Event Creators ============

export function createTableOpenedEvent(data: {
  tableId: string;
  tableName: string;
  zoneId?: string;
  zoneName?: string;
  guestCount: number;
  surcharge?: { type: 'percentage' | 'fixed'; amount: number; name?: string };
  receiptNumber?: string;
}): TableOpenedEvent {
  return createEvent(OrderEventType.TABLE_OPENED, data);
}

export function createItemsAddedEvent(
  items: CartItem[],
  options?: { prePaymentReset?: boolean }
): ItemsAddedEvent {
  return createEvent(OrderEventType.ITEMS_ADDED, { items, prePaymentReset: options?.prePaymentReset });
}

export function createItemModifiedEvent(data: {
  instanceId: string;
  itemName?: string;
  externalId?: string;
  changes: {
    price?: number;
    originalPrice?: number;
    quantity?: number;
    discountPercent?: number;
    surcharge?: number;
    note?: string;
    selectedOptions?: ItemAttributeSelection[];
  };
  previousValues?: Record<string, unknown>;
}): ItemModifiedEvent {
  return createEvent(OrderEventType.ITEM_MODIFIED, data);
}

export function createItemRemovedEvent(data: {
  instanceId: string;
  itemName?: string;
  externalId?: string;
  quantity?: number;
  reason?: string;
}): ItemRemovedEvent {
  return createEvent(OrderEventType.ITEM_REMOVED, data);
}

export function createItemRestoredEvent(instanceId: string): ItemRestoredEvent {
  return createEvent(OrderEventType.ITEM_RESTORED, { instanceId });
}

export function createPaymentAddedEvent(payment: PaymentRecord): PaymentAddedEvent {
  return createEvent(OrderEventType.PAYMENT_ADDED, { payment });
}

export function createPaymentCancelledEvent(data: {
  paymentId: string;
  reason?: string;
}): PaymentCancelledEvent {
  return createEvent(OrderEventType.PAYMENT_CANCELLED, data);
}

export function createOrderCompletedEvent(data: {
  receiptNumber: string;
  finalTotal: number;
}): OrderCompletedEvent {
  return createEvent(OrderEventType.ORDER_COMPLETED, data);
}

export function createOrderVoidedEvent(options?: { reason?: string }): OrderVoidedEvent {
  return createEvent(OrderEventType.ORDER_VOIDED, { reason: options?.reason });
}

export function createOrderRestoredEvent(options?: { reason?: string }): OrderRestoredEvent {
  return createEvent(OrderEventType.ORDER_RESTORED, { reason: options?.reason });
}

export function createOrderSurchargeExemptSetEvent(exempt: boolean): OrderSurchargeExemptSetEvent {
  return createEvent(OrderEventType.ORDER_SURCHARGE_EXEMPT_SET, { exempt });
}

export function createOrderMergedEvent(data: {
  sourceTableId: string;
  sourceTableName: string;
  items: CartItem[];
}): OrderMergedEvent {
  return createEvent(OrderEventType.ORDER_MERGED, data);
}

export function createOrderMovedEvent(data: {
  sourceTableId: string;
  sourceTableName: string;
  targetTableId: string;
  targetTableName: string;
  items: CartItem[];
}): OrderMovedEvent {
  return createEvent(OrderEventType.ORDER_MOVED, data);
}

export function createOrderMovedOutEvent(data: {
  targetTableId: string;
  targetTableName: string;
  reason?: string;
}): OrderMovedOutEvent {
  return createEvent(OrderEventType.ORDER_MOVED_OUT, data);
}

export function createOrderMergedOutEvent(data: {
  targetTableId: string;
  targetTableName: string;
  reason?: string;
}): OrderMergedOutEvent {
  return createEvent(OrderEventType.ORDER_MERGED_OUT, data);
}

export function createTableReassignedEvent(data: {
  sourceTableId: string;
  sourceTableName: string;
  sourceZoneId?: string;
  sourceZoneName?: string;
  targetTableId: string;
  targetTableName: string;
  targetZoneId?: string;
  targetZoneName?: string;
  originalStartTime: number;
  items: CartItem[];
}): TableReassignedEvent {
  return createEvent(OrderEventType.TABLE_REASSIGNED, data);
}

export function createOrderInfoUpdatedEvent(data: {
  receiptNumber?: string;
  guestCount?: number;
  tableName?: string;
  isPrePayment?: boolean;
}): OrderInfoUpdatedEvent {
  return createEvent(OrderEventType.ORDER_INFO_UPDATED, data);
}

export function createOrderSplitEvent(data: {
  splitAmount: number;
  items: Array<{
    instanceId: string;
    name: string;
    quantity: number;
    price: number;
    selectedOptions?: ItemAttributeSelection[];
  }>;
  paymentMethod: string;
  tendered?: number;
  change?: number;
}): OrderSplitEvent {
  return createEvent(OrderEventType.ORDER_SPLIT, data);
}

// ============ Unified Factory Interface ============

export interface OrderEventFactory {
  tableOpened: (data: Parameters<typeof createTableOpenedEvent>[0]) => TableOpenedEvent;
  itemsAdded: (items: CartItem[], options?: { prePaymentReset?: boolean }) => ItemsAddedEvent;
  itemModified: (data: Parameters<typeof createItemModifiedEvent>[0]) => ItemModifiedEvent;
  itemRemoved: (data: Parameters<typeof createItemRemovedEvent>[0]) => ItemRemovedEvent;
  itemRestored: (instanceId: string) => ItemRestoredEvent;
  paymentAdded: (payment: PaymentRecord) => PaymentAddedEvent;
  paymentCancelled: (data: { paymentId: string; reason?: string }) => PaymentCancelledEvent;
  orderCompleted: (data: { receiptNumber: string; finalTotal: number }) => OrderCompletedEvent;
  orderVoided: (options?: { reason?: string }) => OrderVoidedEvent;
  orderRestored: (options?: { reason?: string }) => OrderRestoredEvent;
  surchargeExemptSet: (exempt: boolean) => OrderSurchargeExemptSetEvent;
  orderMerged: (data: Parameters<typeof createOrderMergedEvent>[0]) => OrderMergedEvent;
  orderMoved: (data: Parameters<typeof createOrderMovedEvent>[0]) => OrderMovedEvent;
  orderMovedOut: (data: Parameters<typeof createOrderMovedOutEvent>[0]) => OrderMovedOutEvent;
  orderMergedOut: (data: Parameters<typeof createOrderMergedOutEvent>[0]) => OrderMergedOutEvent;
  tableReassigned: (data: Parameters<typeof createTableReassignedEvent>[0]) => TableReassignedEvent;
  orderInfoUpdated: (data: Parameters<typeof createOrderInfoUpdatedEvent>[0]) => OrderInfoUpdatedEvent;
  orderSplit: (data: Parameters<typeof createOrderSplitEvent>[0]) => OrderSplitEvent;
}

export const eventFactory: OrderEventFactory = {
  tableOpened: createTableOpenedEvent,
  itemsAdded: createItemsAddedEvent,
  itemModified: createItemModifiedEvent,
  itemRemoved: createItemRemovedEvent,
  itemRestored: createItemRestoredEvent,
  paymentAdded: createPaymentAddedEvent,
  paymentCancelled: createPaymentCancelledEvent,
  orderCompleted: createOrderCompletedEvent,
  orderVoided: createOrderVoidedEvent,
  orderRestored: createOrderRestoredEvent,
  surchargeExemptSet: createOrderSurchargeExemptSetEvent,
  orderMerged: createOrderMergedEvent,
  orderMoved: createOrderMovedEvent,
  orderMovedOut: createOrderMovedOutEvent,
  orderMergedOut: createOrderMergedOutEvent,
  tableReassigned: createTableReassignedEvent,
  orderInfoUpdated: createOrderInfoUpdatedEvent,
  orderSplit: createOrderSplitEvent,
};
