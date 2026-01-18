/**
 * Event Sourcing System - Event Reducer (Optimized)
 *
 * This module provides event processing for the order system.
 * Core logic is delegated to adapters.ts for better separation.
 *
 * @deprecated Use functions from adapters.ts directly
 */

export {
  reduceOrderEvents,
  createEmptyOrder,
  recalculateOrderTotal,
  mergeItemsIntoList,
  convertEventToTimelineEvent,
} from '../../domain/events/adapters';

// Re-export types for convenience
export type {
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
} from '../../domain/events/types';
