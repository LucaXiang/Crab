/**
 * Order Stores - Core exports
 */
export { useCheckoutStore } from './useCheckoutStore';
export { useCurrentOrderKey, useCheckoutOrder } from './useCheckoutStore';
export { useDraftOrderStore } from './useDraftOrderStore';

// ============================================================================
// Event Sourcing Architecture - Active Orders (Server-Side State)
// ============================================================================

// Active Orders Store - Main store for server-synced orders
export { useHeldOrders, useHeldOrdersCount } from './useActiveOrdersStore';

// Draft Orders (Local Client State)
export { useDraftOrders, useDraftOrdersCount } from './useDraftOrderStore';

// Order Actions - Combined hook for all order manipulation (draft, checkout, operations)
export { useOrderActions } from './useOrderActions';

// Order Operations - Async command functions
export * as orderOps from './useOrderOperations';

// ============================================================================
// New Event Sourcing Architecture (Server-Side State)
// ============================================================================

// Active Orders Store - Read-only mirror of server state
export {
  useActiveOrdersStore,
  useActiveOrders,
  useOrder,
  useOrderByTable,
  useOrderTimeline,
  useActiveOrderCount,
  useOrderConnectionState,
  useOrdersInitialized,
  useLastSequence,
  useIsOrderConnected,
  useOrderQueries,
  useOrderStoreInternal,
} from './useActiveOrdersStore';

// Order Commands Hook - Send commands to server
export { useOrderCommands } from './useOrderCommands';
export type { OpenTableParams, PaymentInput, OrderCommandsHook } from './useOrderCommands';

// Order Sync Hook - Reconnection and synchronization
export { useOrderSync, setupOrderEventListeners } from './useOrderSync';
export type { OrderSyncHook } from './useOrderSync';

// Order Reducer - Event to snapshot transformation
export { applyEvent, rebuildFromEvents, createEmptySnapshot } from './orderReducer';
