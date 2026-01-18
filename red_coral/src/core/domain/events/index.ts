/**
 * Core Domain Events - Re-export for backward compatibility
 */
export * from './types';
export { OrderEventType } from './types';
export type { OrderEvent, ItemChanges } from './types';

// Event Reducer (optimized)
export {
  reduceOrderEvents,
  createEmptyOrder,
  recalculateOrderTotal,
  mergeItemsIntoList,
} from './adapters';

// Event Factory (type-safe event creation)
export {
  eventFactory,
  createTableOpenedEvent,
  createItemsAddedEvent,
  createItemModifiedEvent,
  createItemRemovedEvent,
  createItemRestoredEvent,
  createPaymentAddedEvent,
  createPaymentCancelledEvent,
  createOrderCompletedEvent,
  createOrderVoidedEvent,
  createOrderRestoredEvent,
  createOrderSurchargeExemptSetEvent,
  createOrderMergedEvent,
  createOrderMovedEvent,
  createOrderMovedOutEvent,
  createOrderMergedOutEvent,
  createTableReassignedEvent,
  createOrderInfoUpdatedEvent,
  createOrderSplitEvent,
} from './factory';
