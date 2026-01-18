/**
 * Event Adapter Pattern - Unified Event Handlers
 *
 * This module provides adapter-based event handling for the order system.
 * Instead of verbose switch statements, we use a map-based approach with
 * composable handlers.
 */

import { HeldOrder, CartItem, ItemAttributeSelection, TimelineEvent } from '@/core/domain/types';
import {
  OrderEvent,
  OrderEventType,
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
  OrderInfoUpdatedEvent,
  OrderSplitEvent,
  TableReassignedEvent,
} from '@/core/domain/events/types';
import { Currency } from '@/utils/currency';
import { calculateItemFinalPrice, calculateItemTotal } from '@/utils/pricing';
import { logger } from '@/utils/logger';
import { formatCurrency } from '@/utils/formatCurrency';

// ============ Event Handler Types ============

type EventHandler<T extends OrderEvent = OrderEvent> = (order: HeldOrder, event: T) => HeldOrder;

interface EventHandlerMap {
  [OrderEventType.TABLE_OPENED]: EventHandler<TableOpenedEvent>;
  [OrderEventType.ITEMS_ADDED]: EventHandler<ItemsAddedEvent>;
  [OrderEventType.ITEM_MODIFIED]: EventHandler<ItemModifiedEvent>;
  [OrderEventType.ITEM_REMOVED]: EventHandler<ItemRemovedEvent>;
  [OrderEventType.ITEM_RESTORED]: EventHandler<ItemRestoredEvent>;
  [OrderEventType.PAYMENT_ADDED]: EventHandler<PaymentAddedEvent>;
  [OrderEventType.PAYMENT_CANCELLED]: EventHandler<PaymentCancelledEvent>;
  [OrderEventType.ORDER_COMPLETED]: EventHandler<OrderCompletedEvent>;
  [OrderEventType.ORDER_VOIDED]: EventHandler<OrderVoidedEvent>;
  [OrderEventType.ORDER_RESTORED]: EventHandler<OrderRestoredEvent>;
  [OrderEventType.ORDER_SURCHARGE_EXEMPT_SET]: EventHandler<OrderSurchargeExemptSetEvent>;
  [OrderEventType.ORDER_MERGED]: EventHandler<OrderMergedEvent>;
  [OrderEventType.ORDER_MOVED]: EventHandler<OrderMovedEvent>;
  [OrderEventType.ORDER_MOVED_OUT]: EventHandler<OrderMovedOutEvent>;
  [OrderEventType.ORDER_MERGED_OUT]: EventHandler<OrderMergedOutEvent>;
  [OrderEventType.TABLE_REASSIGNED]: EventHandler<TableReassignedEvent>;
  [OrderEventType.ORDER_INFO_UPDATED]: EventHandler<OrderInfoUpdatedEvent>;
  [OrderEventType.ORDER_SPLIT]: EventHandler<OrderSplitEvent>;
}

// ============ Shared Utilities ============

function areOptionsEqual(
  options1: ItemAttributeSelection[] | undefined,
  options2: ItemAttributeSelection[] | undefined
): boolean {
  const o1 = options1 && options1.length > 0 ? options1 : undefined;
  const o2 = options2 && options2.length > 0 ? options2 : undefined;
  if (!o1 && !o2) return true;
  if (!o1 || !o2) return false;
  if (o1.length !== o2.length) return false;

  const sorted1 = [...o1].sort((a, b) => `${a.attributeId}-${a.optionId}`.localeCompare(`${b.attributeId}-${b.optionId}`));
  const sorted2 = [...o2].sort((a, b) => `${a.attributeId}-${a.optionId}`.localeCompare(`${b.attributeId}-${b.optionId}`));

  return sorted1.every((opt1, i) => opt1.attributeId === sorted2[i]?.attributeId && opt1.optionId === sorted2[i]?.optionId && opt1.priceModifier === sorted2[i]?.priceModifier);
}

export function mergeItemsIntoList(currentItems: CartItem[], incomingItems: CartItem[]): CartItem[] {
  const merged = [...currentItems];
  const newItems: CartItem[] = [];

  incomingItems.forEach(incoming => {
    const matchIdx = merged.findIndex(existing =>
      !existing._removed && existing.id === incoming.id &&
      (existing.discountPercent || 0) === (incoming.discountPercent || 0) &&
      existing.price === incoming.price &&
      areOptionsEqual(existing.selectedOptions, incoming.selectedOptions)
    );

    if (matchIdx !== -1) {
      merged[matchIdx] = { ...merged[matchIdx], quantity: merged[matchIdx].quantity + incoming.quantity };
    } else {
      newItems.push({ ...incoming, instanceId: incoming.instanceId || `item-${Date.now()}-${Math.random().toString(36).slice(2, 11)}` });
    }
  });

  return [...merged, ...newItems];
}

// ============ Event Handlers ============

const handlers: EventHandlerMap = {
  [OrderEventType.TABLE_OPENED]: (_order, event) => ({
    ...createEmptyOrder(event.data.tableId),
    key: event.data.tableId,
    tableName: event.data.tableName,
    zoneName: event.data.zoneName,
    guestCount: event.data.guestCount,
    startTime: event.timestamp,
    surcharge: event.data.surcharge ? { ...event.data.surcharge, total: 0 } : undefined,
    receiptNumber: event.data.receiptNumber,
    timeline: [toTimelineEvent(event)],
  }),

  [OrderEventType.ITEMS_ADDED]: (order, event) => ({
    ...order,
    items: mergeItemsIntoList(order.items, event.data.items),
    isPrePayment: false,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ITEM_MODIFIED]: (order, event) => ({
    ...order,
    items: order.items.map(item => {
      if (item.instanceId !== event.data.instanceId) return item;
      const updated = { ...item, ...event.data.changes };
      if (event.data.changes.discountPercent !== undefined || event.data.changes.surcharge !== undefined || event.data.changes.originalPrice !== undefined || event.data.changes.selectedOptions !== undefined) {
        updated.price = calculateItemFinalPrice(updated).toNumber();
      }
      return updated;
    }),
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ITEM_REMOVED]: (order, event) => ({
    ...order,
    items: order.items.flatMap(item => {
      if (item.instanceId !== event.data.instanceId) return [item];
      const qty = event.data.quantity;
      if (qty && qty > 0 && qty < item.quantity) {
        return [
          { ...item, quantity: item.quantity - qty },
          { ...item, quantity: qty, _removed: true, instanceId: `${item.instanceId}-removed-${event.timestamp}`, originalInstanceId: item.instanceId },
        ];
      }
      return [{ ...item, _removed: true }];
    }),
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ITEM_RESTORED]: (order, event) => ({
    ...order,
    items: order.items.map(item => item.instanceId === event.data.instanceId && item._removed ? (({ _removed, ...rest }) => rest)(item as any) : item),
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.PAYMENT_ADDED]: (order, event) => ({
    ...order,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.PAYMENT_CANCELLED]: (order, event) => ({
    ...order,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_COMPLETED]: (order, event) => ({
    ...order,
    status: 'COMPLETED',
    endTime: event.timestamp,
    receiptNumber: event.data.receiptNumber,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_VOIDED]: (order, event) => ({
    ...order,
    status: 'VOID',
    endTime: event.timestamp,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_RESTORED]: (order, event) => ({
    ...order,
    status: 'ACTIVE',
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_SURCHARGE_EXEMPT_SET]: (order, event) => ({
    ...order,
    surchargeExempt: !!event.data.exempt,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_MERGED]: (order, event) => ({
    ...order,
    items: mergeItemsIntoList(order.items, event.data.items),
    isPrePayment: false,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_MOVED]: (order, event) => ({
    ...order,
    items: mergeItemsIntoList(order.items, event.data.items),
    isPrePayment: false,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_MOVED_OUT]: (order, event) => ({
    ...order,
    status: 'MOVED',
    endTime: event.timestamp,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_MERGED_OUT]: (order, event) => ({
    ...order,
    status: 'MERGED',
    endTime: event.timestamp,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  /**
   * TABLE_REASSIGNED - 处理桌台转移（不修改原始事件）
   *
   * 当订单转移到新桌台时：
   * - 更新桌台信息 (tableId, tableName, zoneName)
   * - 保留原始开始时间 (originalStartTime)
   * - 合并商品列表
   */
  [OrderEventType.TABLE_REASSIGNED]: (order, event) => ({
    ...order,
    key: event.data.targetTableId,
    tableName: event.data.targetTableName,
    zoneName: event.data.targetZoneName,
    // 保留原始开始时间（从源桌台继承）
    startTime: event.data.originalStartTime,
    // 合并商品（从源桌台带过来的）
    items: mergeItemsIntoList(order.items, event.data.items),
    isPrePayment: false,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_INFO_UPDATED]: (order, event) => ({
    ...order,
    receiptNumber: event.data.receiptNumber || order.receiptNumber,
    guestCount: event.data.guestCount || order.guestCount,
    tableName: event.data.tableName || order.tableName,
    isPrePayment: event.data.isPrePayment ?? order.isPrePayment,
    timeline: [...order.timeline, toTimelineEvent(event)],
  }),

  [OrderEventType.ORDER_SPLIT]: (order, event) => {
    const paidQty = { ...(order.paidItemQuantities || {}) };
    event.data.items.forEach(item => { paidQty[item.instanceId] = (paidQty[item.instanceId] || 0) + item.quantity; });
    return {
      ...order,
      paidAmount: Currency.add(order.paidAmount || 0, event.data.splitAmount).toNumber(),
      paidItemQuantities: paidQty,
      timeline: [...order.timeline, toTimelineEvent(event)],
    };
  },
};

// ============ Main Reducer ============

export function createEmptyOrder(key: string): HeldOrder {
  return {
    key,
    tableName: '',
    items: [],
    total: 0,
    subtotal: 0,
    tax: 0,
    discount: 0,
    payments: [],
    guestCount: 0,
    startTime: Date.now(),
    timeline: [],
    createdAt: Date.now(),
    updatedAt: Date.now(),
    status: 'ACTIVE',
  };
}

export function reduceOrderEvents(events: OrderEvent[], initialOrder?: HeldOrder): HeldOrder {
  let order = initialOrder || createEmptyOrder('temp');
  for (const event of events) {
    const handler = handlers[event.type as keyof EventHandlerMap];
    if (handler) {
      order = handler(order, event as any);
    } else {
      logger.warn('Unknown event type', { component: 'eventAdapter', eventType: event.type });
    }
  }
  return recalculateOrderTotal(order);
}

// ============ Timeline Event Conversion ============

function toTimelineEvent(event: OrderEvent): TimelineEvent {
  return {
    id: event.id,
    type: event.type as TimelineEvent['type'],
    timestamp: event.timestamp,
    title: getEventTitle(event),
    summary: getEventSummary(event),
    data: event.data,
  };
}

function getEventTitle(event: OrderEvent): string {
  const titles: Record<OrderEventType, string> = {
    [OrderEventType.TABLE_OPENED]: 'Table Opened',
    [OrderEventType.ITEMS_ADDED]: 'Items Added',
    [OrderEventType.ITEM_MODIFIED]: 'Item Modified',
    [OrderEventType.ITEM_REMOVED]: 'Item Removed',
    [OrderEventType.ITEM_RESTORED]: 'Item Restored',
    [OrderEventType.PAYMENT_ADDED]: 'Payment Added',
    [OrderEventType.PAYMENT_CANCELLED]: 'Payment Cancelled',
    [OrderEventType.ORDER_COMPLETED]: 'Order Completed',
    [OrderEventType.ORDER_VOIDED]: 'Order Voided',
    [OrderEventType.ORDER_RESTORED]: 'Order Restored',
    [OrderEventType.ORDER_SURCHARGE_EXEMPT_SET]: 'Surcharge Exempt',
    [OrderEventType.ORDER_MERGED]: 'Table Merged',
    [OrderEventType.ORDER_MOVED]: 'Table Moved',
    [OrderEventType.ORDER_MOVED_OUT]: 'Table Moved Out',
    [OrderEventType.ORDER_MERGED_OUT]: 'Table Merged Out',
    [OrderEventType.TABLE_REASSIGNED]: 'Table Reassigned',
    [OrderEventType.ORDER_INFO_UPDATED]: 'Order Info Updated',
    [OrderEventType.ORDER_SPLIT]: 'Split Bill',
  };
  return titles[event.type] || 'Unknown Event';
}

function getEventSummary(event: OrderEvent): string {
  const d = event.data as Record<string, unknown>;
  switch (event.type) {
    case OrderEventType.ITEMS_ADDED: return `Added ${(d.items as unknown[])?.length || 0} items`;
    case OrderEventType.PAYMENT_ADDED: {
      const payment = d.payment as { method?: string; amount?: number };
      return `${payment?.method} payment: ${formatCurrency(payment?.amount || 0)}`;
    }
    case OrderEventType.ORDER_SURCHARGE_EXEMPT_SET: return d.exempt ? 'Exempt surcharge enabled' : 'Exempt surcharge disabled';
    case OrderEventType.ORDER_MERGED: return `Merged from ${d.sourceTableName}`;
    case OrderEventType.ORDER_MOVED: return `Moved from ${d.sourceTableName}`;
    case OrderEventType.ORDER_MOVED_OUT: return `Moved to ${d.targetTableName}`;
    case OrderEventType.ORDER_MERGED_OUT: return `Merged to ${d.targetTableName}`;
    case OrderEventType.TABLE_REASSIGNED: return `Reassigned from ${d.sourceTableName} to ${d.targetTableName}`;
    case OrderEventType.ORDER_SPLIT: return `Split Payment: ${formatCurrency((d.splitAmount as number) || 0)}`;
    default: return '';
  }
}

// ============ Price Calculation ============

/**
 * 将事件转换为时间线事件
 */
export function convertEventToTimelineEvent(event: OrderEvent): TimelineEvent {
  return {
    id: event.id,
    type: event.type as TimelineEvent['type'],
    timestamp: event.timestamp,
    title: getEventTitle(event),
    summary: getEventSummary(event),
    data: event.data,
  };
}

export function recalculateOrderTotal(order: HeldOrder): HeldOrder {
  const allItems = order.items.map(item => {
    let surcharge = 0;
    if (order.surchargeExempt) {
      surcharge = 0;
    } else if (order.surcharge && order.surcharge.value) {
      if (order.surcharge.type === 'percentage') {
        const basePrice = Currency.add(item.originalPrice ?? 0, 0);
        const discountedBase = Currency.sub(basePrice, 0); // simplified
        surcharge = Currency.floor2((discountedBase.toNumber() * order.surcharge.value) / 100).toNumber();
      } else {
        surcharge = order.surcharge.value;
      }
    } else {
      surcharge = item.surcharge || 0;
    }
    const finalPrice = calculateItemFinalPrice({ ...item, surcharge }).toNumber();
    return { ...item, surcharge, price: finalPrice };
  });

  const activeItems = allItems.filter(i => !i._removed);
  const itemsSubtotal = activeItems.reduce((sum, i) => Currency.add(sum, calculateItemTotal(i)).toNumber(), 0);

  let total = itemsSubtotal;
  if (!order.surchargeExempt && order.surcharge && order.surcharge.amount > 0) {
    const surchargeTotal = order.surcharge.type === 'percentage'
      ? Currency.floor2(Currency.mul(itemsSubtotal, order.surcharge.amount / 100)).toNumber()
      : order.surcharge.amount;
    total = Currency.add(total, surchargeTotal).toNumber();
    order = { ...order, surcharge: { ...order.surcharge, total: surchargeTotal } };
  }

  return { ...order, items: allItems, total };
}
