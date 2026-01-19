/**
 * Event Types
 * Defines all events used in the order event sourcing system
 */

/**
 * Base event type
 */
export interface BaseEvent {
  id: string;
  type: OrderEventType;
  timestamp: string;
  userId?: string;
  userName?: string;
}

/**
 * All order event types
 */
export type OrderEventType =
  | 'ORDER_CREATED'
  | 'ITEM_ADDED'
  | 'ITEM_MODIFIED'
  | 'ITEM_REMOVED'
  | 'ITEM_SWAPPED'
  | 'DISCOUNT_APPLIED'
  | 'DISCOUNT_REMOVED'
  | 'NOTE_ADDED'
  | 'NOTE_REMOVED'
  | 'TABLE_ASSIGNED'
  | 'TABLE_RELEASED'
  | 'GUEST_COUNT_UPDATED'
  | 'STATUS_CHANGED'
  | 'PAYMENT_INITIATED'
  | 'PAYMENT_COMPLETED'
  | 'PAYMENT_FAILED'
  | 'PAYMENT_REFUNDED'
  | 'ORDER_VOIDED'
  | 'ORDER_COMPLETED'
  | 'ORDER_REOPENED'
  | 'MERGE_INITIATED'
  | 'MERGE_COMPLETED'
  | 'SPLIT_INITIATED'
  | 'SPLIT_COMPLETED'
  | 'EXTERNAL_ID_ASSIGNED'
  | 'PRINT_KITCHEN_TICKET'
  | 'PRINT_RECEIPT'
  | 'CUSTOM_ACTION';

/**
 * Order created event
 */
export interface OrderCreatedEvent extends BaseEvent {
  type: 'ORDER_CREATED';
  payload: {
    orderId: string;
    tableId?: string;
    tableName?: string;
    guestCount?: number;
    source: 'pos' | 'table' | 'history';
  };
}

/**
 * Item added to order event
 */
export interface ItemAddedEvent extends BaseEvent {
  type: 'ITEM_ADDED';
  payload: {
    itemId: string;
    productId: string;
    productName: string;
    quantity: number;
    unitPrice: number;
    finalPrice: number;
    options?: ItemEventOption[];
    note?: string;
  };
}

/**
 * Item modification event
 */
export interface ItemModifiedEvent extends BaseEvent {
  type: 'ITEM_MODIFIED';
  payload: {
    itemId: string;
    changes: {
      quantity?: number;
      unitPrice?: number;
      finalPrice?: number;
      options?: ItemEventOption[];
      note?: string;
    };
    previousPrice?: number;
  };
}

/**
 * Item removed from order event
 */
export interface ItemRemovedEvent extends BaseEvent {
  type: 'ITEM_REMOVED';
  payload: {
    itemId: string;
    productId: string;
    productName: string;
    quantity: number;
    reason?: string;
  };
}

/**
 * Item swapped event (e.g., swapped modifier)
 */
export interface ItemSwappedEvent extends BaseEvent {
  type: 'ITEM_SWAPPED';
  payload: {
    itemId: string;
    originalOptionId: string;
    newOptionId: string;
    priceDifference: number;
  };
}

/**
 * Discount applied event
 */
export interface DiscountAppliedEvent extends BaseEvent {
  type: 'DISCOUNT_APPLIED';
  payload: {
    discountType: 'order' | 'item';
    discountTarget: string;  // orderId or itemId
    discountName: string;
    discountValue: number;
    discountMethod: 'percentage' | 'fixed';
    appliedAmount: number;
  };
}

/**
 * Discount removed event
 */
export interface DiscountRemovedEvent extends BaseEvent {
  type: 'DISCOUNT_REMOVED';
  payload: {
    discountTarget: string;
    discountName: string;
    originalAmount: number;
  };
}

/**
 * Note added event
 */
export interface NoteAddedEvent extends BaseEvent {
  type: 'NOTE_ADDED';
  payload: {
    targetType: 'order' | 'item';
    targetId: string;
    note: string;
  };
}

/**
 * Note removed event
 */
export interface NoteRemovedEvent extends BaseEvent {
  type: 'NOTE_REMOVED';
  payload: {
    targetType: 'order' | 'item';
    targetId: string;
    noteId: string;
  };
}

/**
 * Table assigned event
 */
export interface TableAssignedEvent extends BaseEvent {
  type: 'TABLE_ASSIGNED';
  payload: {
    tableId: string;
    tableName: string;
    previousTableId?: string;
  };
}

/**
 * Table released event
 */
export interface TableReleasedEvent extends BaseEvent {
  type: 'TABLE_RELEASED';
  payload: {
    tableId: string;
    tableName: string;
  };
}

/**
 * Guest count updated event
 */
export interface GuestCountUpdatedEvent extends BaseEvent {
  type: 'GUEST_COUNT_UPDATED';
  payload: {
    previousCount: number;
    currentCount: number;
  };
}

/**
 * Order status changed event
 */
export interface StatusChangedEvent extends BaseEvent {
  type: 'STATUS_CHANGED';
  payload: {
    previousStatus: string;
    currentStatus: string;
    reason?: string;
  };
}

/**
 * Payment initiated event
 */
export interface PaymentInitiatedEvent extends BaseEvent {
  type: 'PAYMENT_INITIATED';
  payload: {
    paymentId: string;
    amount: number;
    method: string;
  };
}

/**
 * Payment completed event
 */
export interface PaymentCompletedEvent extends BaseEvent {
  type: 'PAYMENT_COMPLETED';
  payload: {
    paymentId: string;
    amount: number;
    method: string;
    change?: number;
    tip?: number;
  };
}

/**
 * Payment failed event
 */
export interface PaymentFailedEvent extends BaseEvent {
  type: 'PAYMENT_FAILED';
  payload: {
    paymentId: string;
    reason: string;
    errorCode?: string;
  };
}

/**
 * Payment refunded event
 */
export interface PaymentRefundedEvent extends BaseEvent {
  type: 'PAYMENT_REFUNDED';
  payload: {
    paymentId: string;
    amount: number;
    reason: string;
  };
}

/**
 * Order voided event
 */
export interface OrderVoidedEvent extends BaseEvent {
  type: 'ORDER_VOIDED';
  payload: {
    reason: string;
    supervisorId?: string;
    supervisorName?: string;
  };
}

/**
 * Order completed event
 */
export interface OrderCompletedEvent extends BaseEvent {
  type: 'ORDER_COMPLETED';
  payload: {
    totalAmount: number;
    paymentMethod: string;
    duration: number;  // in seconds
  };
}

/**
 * Order reopened event
 */
export interface OrderReopenedEvent extends BaseEvent {
  type: 'ORDER_REOPENED';
  payload: {
    reason: string;
  };
}

/**
 * Order merge initiated event
 */
export interface MergeInitiatedEvent extends BaseEvent {
  type: 'MERGE_INITIATED';
  payload: {
    sourceOrderId: string;
    targetOrderId: string;
  };
}

/**
 * Order merge completed event
 */
export interface MergeCompletedEvent extends BaseEvent {
  type: 'MERGE_COMPLETED';
  payload: {
    sourceOrderId: string;
    targetOrderId: string;
    mergedItems: string[];
  };
}

/**
 * Order split initiated event
 */
export interface SplitInitiatedEvent extends BaseEvent {
  type: 'SPLIT_INITIATED';
  payload: {
    orderId: string;
    splitItems: string[];
    targetOrderId: string;
  };
}

/**
 * Order split completed event
 */
export interface SplitCompletedEvent extends BaseEvent {
  type: 'SPLIT_COMPLETED';
  payload: {
    orderId: string;
    splitOrderId: string;
    items: string[];
  };
}

/**
 * External ID assigned event
 */
export interface ExternalIdAssignedEvent extends BaseEvent {
  type: 'EXTERNAL_ID_ASSIGNED';
  payload: {
    externalId: string;
    externalSystem: string;
  };
}

/**
 * Print kitchen ticket event
 */
export interface PrintKitchenTicketEvent extends BaseEvent {
  type: 'PRINT_KITCHEN_TICKET';
  payload: {
    items: string[];
    printerId?: number;
  };
}

/**
 * Print receipt event
 */
export interface PrintReceiptEvent extends BaseEvent {
  type: 'PRINT_RECEIPT';
  payload: {
    receiptNumber: string;
    copyType: 'original' | 'copy' | 'reprint';
  };
}

/**
 * Custom action event
 */
export interface CustomActionEvent extends BaseEvent {
  type: 'CUSTOM_ACTION';
  payload: {
    action: string;
    data: Record<string, any>;
  };
}

/**
 * Item event option (used in ItemAdded/ItemModified events)
 */
export interface ItemEventOption {
  attributeId: string;
  attributeName: string;
  optionId: string;
  optionName: string;
  priceModifier: number;
}

/**
 * Union type for all order events
 */
export type OrderEvent =
  | OrderCreatedEvent
  | ItemAddedEvent
  | ItemModifiedEvent
  | ItemRemovedEvent
  | ItemSwappedEvent
  | DiscountAppliedEvent
  | DiscountRemovedEvent
  | NoteAddedEvent
  | NoteRemovedEvent
  | TableAssignedEvent
  | TableReleasedEvent
  | GuestCountUpdatedEvent
  | StatusChangedEvent
  | PaymentInitiatedEvent
  | PaymentCompletedEvent
  | PaymentFailedEvent
  | PaymentRefundedEvent
  | OrderVoidedEvent
  | OrderCompletedEvent
  | OrderReopenedEvent
  | MergeInitiatedEvent
  | MergeCompletedEvent
  | SplitInitiatedEvent
  | SplitCompletedEvent
  | ExternalIdAssignedEvent
  | PrintKitchenTicketEvent
  | PrintReceiptEvent
  | CustomActionEvent;

/**
 * Event metadata for display
 */
export interface EventMetadata {
  icon: string;
  color: string;
  label: string;
  description: string;
}

/**
 * Event type keys for i18n lookup
 */
const EVENT_TYPE_KEYS: Record<OrderEventType, { label: string; description: string }> = {
  ORDER_CREATED: { label: 'events.ORDER_CREATED.label', description: 'events.ORDER_CREATED.description' },
  ITEM_ADDED: { label: 'events.ITEM_ADDED.label', description: 'events.ITEM_ADDED.description' },
  ITEM_MODIFIED: { label: 'events.ITEM_MODIFIED.label', description: 'events.ITEM_MODIFIED.description' },
  ITEM_REMOVED: { label: 'events.ITEM_REMOVED.label', description: 'events.ITEM_REMOVED.description' },
  ITEM_SWAPPED: { label: 'events.ITEM_SWAPPED.label', description: 'events.ITEM_SWAPPED.description' },
  DISCOUNT_APPLIED: { label: 'events.DISCOUNT_APPLIED.label', description: 'events.DISCOUNT_APPLIED.description' },
  DISCOUNT_REMOVED: { label: 'events.DISCOUNT_REMOVED.label', description: 'events.DISCOUNT_REMOVED.description' },
  NOTE_ADDED: { label: 'events.NOTE_ADDED.label', description: 'events.NOTE_ADDED.description' },
  NOTE_REMOVED: { label: 'events.NOTE_REMOVED.label', description: 'events.NOTE_REMOVED.description' },
  TABLE_ASSIGNED: { label: 'events.TABLE_ASSIGNED.label', description: 'events.TABLE_ASSIGNED.description' },
  TABLE_RELEASED: { label: 'events.TABLE_RELEASED.label', description: 'events.TABLE_RELEASED.description' },
  GUEST_COUNT_UPDATED: { label: 'events.GUEST_COUNT_UPDATED.label', description: 'events.GUEST_COUNT_UPDATED.description' },
  STATUS_CHANGED: { label: 'events.STATUS_CHANGED.label', description: 'events.STATUS_CHANGED.description' },
  PAYMENT_INITIATED: { label: 'events.PAYMENT_INITIATED.label', description: 'events.PAYMENT_INITIATED.description' },
  PAYMENT_COMPLETED: { label: 'events.PAYMENT_COMPLETED.label', description: 'events.PAYMENT_COMPLETED.description' },
  PAYMENT_FAILED: { label: 'events.PAYMENT_FAILED.label', description: 'events.PAYMENT_FAILED.description' },
  PAYMENT_REFUNDED: { label: 'events.PAYMENT_REFUNDED.label', description: 'events.PAYMENT_REFUNDED.description' },
  ORDER_VOIDED: { label: 'events.ORDER_VOIDED.label', description: 'events.ORDER_VOIDED.description' },
  ORDER_COMPLETED: { label: 'events.ORDER_COMPLETED.label', description: 'events.ORDER_COMPLETED.description' },
  ORDER_REOPENED: { label: 'events.ORDER_REOPENED.label', description: 'events.ORDER_REOPENED.description' },
  MERGE_INITIATED: { label: 'events.MERGE_INITIATED.label', description: 'events.MERGE_INITIATED.description' },
  MERGE_COMPLETED: { label: 'events.MERGE_COMPLETED.label', description: 'events.MERGE_COMPLETED.description' },
  SPLIT_INITIATED: { label: 'events.SPLIT_INITIATED.label', description: 'events.SPLIT_INITIATED.description' },
  SPLIT_COMPLETED: { label: 'events.SPLIT_COMPLETED.label', description: 'events.SPLIT_COMPLETED.description' },
  EXTERNAL_ID_ASSIGNED: { label: 'events.EXTERNAL_ID_ASSIGNED.label', description: 'events.EXTERNAL_ID_ASSIGNED.description' },
  PRINT_KITCHEN_TICKET: { label: 'events.PRINT_KITCHEN_TICKET.label', description: 'events.PRINT_KITCHEN_TICKET.description' },
  PRINT_RECEIPT: { label: 'events.PRINT_RECEIPT.label', description: 'events.PRINT_RECEIPT.description' },
  CUSTOM_ACTION: { label: 'events.CUSTOM_ACTION.label', description: 'events.CUSTOM_ACTION.description' },
};

/**
 * Icon/color mapping for event types (these don't need translation)
 */
const EVENT_ICONS: Partial<Record<OrderEventType, { icon: string; color: string }>> = {
  ORDER_CREATED: { icon: 'plus', color: 'blue' },
  ITEM_ADDED: { icon: 'cart-plus', color: 'green' },
  ITEM_MODIFIED: { icon: 'edit', color: 'yellow' },
  ITEM_REMOVED: { icon: 'trash', color: 'red' },
  ITEM_SWAPPED: { icon: 'swap', color: 'orange' },
  DISCOUNT_APPLIED: { icon: 'percent', color: 'purple' },
  DISCOUNT_REMOVED: { icon: 'x-percent', color: 'gray' },
  NOTE_ADDED: { icon: 'message', color: 'blue' },
  NOTE_REMOVED: { icon: 'x', color: 'gray' },
  TABLE_ASSIGNED: { icon: 'layout', color: 'blue' },
  TABLE_RELEASED: { icon: 'layout', color: 'gray' },
  GUEST_COUNT_UPDATED: { icon: 'users', color: 'blue' },
  STATUS_CHANGED: { icon: 'flag', color: 'yellow' },
  PAYMENT_INITIATED: { icon: 'credit-card', color: 'blue' },
  PAYMENT_COMPLETED: { icon: 'check-circle', color: 'green' },
  PAYMENT_FAILED: { icon: 'x-circle', color: 'red' },
  PAYMENT_REFUNDED: { icon: 'rotate-ccw', color: 'orange' },
  ORDER_VOIDED: { icon: 'ban', color: 'red' },
  ORDER_COMPLETED: { icon: 'check', color: 'green' },
  ORDER_REOPENED: { icon: 'rotate-ccw', color: 'blue' },
  MERGE_INITIATED: { icon: 'git-merge', color: 'purple' },
  MERGE_COMPLETED: { icon: 'git-merge', color: 'green' },
  SPLIT_INITIATED: { icon: 'git-branch', color: 'purple' },
  SPLIT_COMPLETED: { icon: 'git-branch', color: 'green' },
  EXTERNAL_ID_ASSIGNED: { icon: 'link', color: 'blue' },
  PRINT_KITCHEN_TICKET: { icon: 'printer', color: 'blue' },
  PRINT_RECEIPT: { icon: 'receipt', color: 'blue' },
  CUSTOM_ACTION: { icon: 'cog', color: 'gray' },
};

/**
 * Get translation keys for an event type
 */
export function getEventTypeKeys(type: OrderEventType): { label: string; description: string } {
  return EVENT_TYPE_KEYS[type] || { label: `events.${type}.label`, description: `events.${type}.description` };
}

/**
 * Get icon and color for an event type
 */
export function getEventIcon(type: OrderEventType): { icon: string; color: string } {
  return EVENT_ICONS[type] || { icon: 'circle', color: 'gray' };
}

/**
 * Get metadata for an event type (fallback for non-i18n contexts)
 * Use getEventTypeKeys with t() for translated versions
 */
export function getEventMetadata(type: OrderEventType): EventMetadata {
  const keys = EVENT_TYPE_KEYS[type] || { label: type, description: type };
  const iconInfo = EVENT_ICONS[type] || { icon: 'circle', color: 'gray' };
  return {
    ...iconInfo,
    label: keys.label,
    description: keys.description,
  };
}
