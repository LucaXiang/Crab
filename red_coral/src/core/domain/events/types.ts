/**
 * Event Sourcing System - Event Type Definitions
 *
 * All order operations are recorded as events.
 * Order state is computed by replaying events.
 */

import { CartItem, PaymentRecord } from '@/core/domain/types';

// ============ Event Type Enum ============

export enum OrderEventType {
  // Order lifecycle
  TABLE_OPENED = 'TABLE_OPENED',
  ORDER_COMPLETED = 'ORDER_COMPLETED',
  ORDER_VOIDED = 'ORDER_VOIDED',
  ORDER_RESTORED = 'ORDER_RESTORED',

  // Item operations
  ITEMS_ADDED = 'ITEMS_ADDED',
  ITEM_MODIFIED = 'ITEM_MODIFIED',
  ITEM_REMOVED = 'ITEM_REMOVED',
  ITEM_RESTORED = 'ITEM_RESTORED',

  // Payment operations
  PAYMENT_ADDED = 'PAYMENT_ADDED',
  PAYMENT_CANCELLED = 'PAYMENT_CANCELLED',

  ORDER_SURCHARGE_EXEMPT_SET = 'ORDER_SURCHARGE_EXEMPT_SET',

  // Table Management
  ORDER_MERGED = 'ORDER_MERGED',
  ORDER_MOVED = 'ORDER_MOVED',
  ORDER_MOVED_OUT = 'ORDER_MOVED_OUT',
  ORDER_MERGED_OUT = 'ORDER_MERGED_OUT',

  // Table relocation (preserves event immutability)
  TABLE_REASSIGNED = 'TABLE_REASSIGNED',

  // Order info updates
  ORDER_INFO_UPDATED = 'ORDER_INFO_UPDATED',
  ORDER_SPLIT = 'ORDER_SPLIT',
}

// ============ Event Data Structures ============

interface BaseOrderEvent {
  id: string;
  type: OrderEventType;
  timestamp: number;
  userId?: string;
  note?: string;
  title?: string;
  summary?: string;
}

export interface TableOpenedEvent extends BaseOrderEvent {
  type: OrderEventType.TABLE_OPENED;
  data: {
    tableId: string;
    tableName: string;
    zoneId?: string;
    zoneName?: string;
    guestCount: number;
    surcharge?: {
      type: 'percentage' | 'fixed';
      amount: number;
      name?: string;
    };
    receiptNumber?: string;
  };
}

export interface ItemsAddedEvent extends BaseOrderEvent {
  type: OrderEventType.ITEMS_ADDED;
  data: {
    items: CartItem[];
    prePaymentReset?: boolean;
  };
}

export interface ItemModifiedEvent extends BaseOrderEvent {
  type: OrderEventType.ITEM_MODIFIED;
  data: {
    instanceId: string;
    itemName?: string;
    externalId?: string;
    changes: ItemChanges;
    previousValues?: Partial<ItemChanges>;
  };
}

export type ItemChanges = {
  price?: number;
  originalPrice?: number;
  quantity?: number;
  discountPercent?: number;
  surcharge?: number;
  note?: string;
  selectedOptions?: import('../types').ItemAttributeSelection[];
};

export interface ItemRemovedEvent extends BaseOrderEvent {
  type: OrderEventType.ITEM_REMOVED;
  data: {
    instanceId: string;
    itemName?: string;
    externalId?: string;
    quantity?: number;
    reason?: string;
  };
}

export interface ItemRestoredEvent extends BaseOrderEvent {
  type: OrderEventType.ITEM_RESTORED;
  data: {
    instanceId: string;
  };
}

export interface PaymentAddedEvent extends BaseOrderEvent {
  type: OrderEventType.PAYMENT_ADDED;
  data: {
    payment: PaymentRecord;
  };
}

export interface PaymentCancelledEvent extends BaseOrderEvent {
  type: OrderEventType.PAYMENT_CANCELLED;
  data: {
    paymentId: string;
    reason?: string;
  };
}

export interface OrderSurchargeExemptSetEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_SURCHARGE_EXEMPT_SET;
  data: {
    exempt: boolean;
    surcharge?: {
      type: 'percentage' | 'fixed';
      amount: number;
      total: number;
      value?: number;
      name?: string;
    };
  };
}

export interface OrderMergedEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_MERGED;
  data: {
    sourceTableId: string;
    sourceTableName: string;
    items: CartItem[];
  };
}

export interface OrderMovedEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_MOVED;
  data: {
    sourceTableId: string;
    sourceTableName: string;
    targetTableId: string;
    targetTableName: string;
    items: CartItem[];
  };
}

export interface OrderMovedOutEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_MOVED_OUT;
  data: {
    targetTableId: string;
    targetTableName: string;
    reason?: string;
  };
}

export interface OrderMergedOutEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_MERGED_OUT;
  data: {
    targetTableId: string;
    targetTableName: string;
    reason?: string;
  };
}

export interface OrderInfoUpdatedEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_INFO_UPDATED;
  data: {
    receiptNumber?: string;
    guestCount?: number;
    tableName?: string;
    isPrePayment?: boolean;
  };
}

export interface OrderSplitEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_SPLIT;
  data: {
    splitAmount: number;
    items: {
      instanceId: string;
      name: string;
      quantity: number;
      price: number;
      selectedOptions?: import('@/core/domain/types').ItemAttributeSelection[];
    }[];
    paymentMethod: string;
    tendered?: number;
    change?: number;
  };
}

export interface OrderCompletedEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_COMPLETED;
  data: {
    receiptNumber: string;
    finalTotal: number;
  };
}

export interface OrderVoidedEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_VOIDED;
  data: {
    reason?: string;
  };
}

export interface OrderRestoredEvent extends BaseOrderEvent {
  type: OrderEventType.ORDER_RESTORED;
  data: {
    reason?: string;
  };
}

/**
 * Table Reassignment Event - 用于桌台转移，保留事件不可变性
 *
 * 当订单从源桌台转移到目标桌台时：
 * 1. 源订单添加 ORDER_MOVED_OUT 事件（标记为已移动）
 * 2. 目标订单添加 TABLE_REASSIGNED 事件（记录源桌台信息）
 *
 * 注意：不再修改原始 TABLE_OPENED 事件，保持事件溯源的不可变性
 */
export interface TableReassignedEvent extends BaseOrderEvent {
  type: OrderEventType.TABLE_REASSIGNED;
  data: {
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
  };
}

// ============ Union Type ============

export type OrderEvent =
  | TableOpenedEvent
  | ItemsAddedEvent
  | ItemModifiedEvent
  | ItemRemovedEvent
  | ItemRestoredEvent
  | PaymentAddedEvent
  | PaymentCancelledEvent
  | OrderCompletedEvent
  | OrderVoidedEvent
  | OrderRestoredEvent
  | OrderSurchargeExemptSetEvent
  | OrderMergedEvent
  | OrderMovedEvent
  | OrderMovedOutEvent
  | OrderMergedOutEvent
  | TableReassignedEvent
  | OrderInfoUpdatedEvent
  | OrderSplitEvent;

// ============ Helper Functions ============

export function createEvent<T extends OrderEvent>(
  type: OrderEventType,
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
