/**
 * Core Domain Events
 *
 * Frontend event types for order state management.
 * These are camelCase versions of the backend snake_case events.
 */

// Type exports
export * from './types';
export { OrderEventType } from './types';
export type { OrderEvent, ItemChanges } from './types';

// Event reducer functions (used for local state management)
export {
  reduceOrderEvents,
  createEmptyOrder,
  recalculateOrderTotal,
  mergeItemsIntoList,
  convertEventToTimelineEvent,
} from './adapters';
