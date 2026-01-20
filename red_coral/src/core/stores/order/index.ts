/**
 * Order Stores - Core exports
 */
export { useCheckoutStore } from './useCheckoutStore';
export { usePaymentStore } from './usePaymentStore';
export { useHeldOrders } from './useOrderStore';
export { useHeldOrdersCount } from './useOrderStore';
export { useDraftOrders } from './useOrderStore';
export { useDraftOrdersCount } from './useOrderStore';
export { useOrderActions } from './useOrderStore';
export { useCurrentOrderKey, useCheckoutOrder } from './useCheckoutStore';
export { useDraftOrderStore } from './useDraftOrderStore';
export { useReceiptStore } from './useReceiptStore';

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
