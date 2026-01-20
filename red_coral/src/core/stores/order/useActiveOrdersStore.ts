/**
 * Active Orders Store (Read-Only Mirror)
 *
 * This store maintains a read-only mirror of server-side order state.
 * All state changes come from server events - no local mutations allowed.
 *
 * Architecture:
 * - State is updated only through internal methods (_applyEvent, _fullSync)
 * - These methods are called by event listeners in App.tsx
 * - UI components read state through selectors
 * - Commands are sent via useOrderCommands hook (separate file)
 *
 * Event Flow:
 * 1. Server broadcasts OrderEvent
 * 2. Tauri emits 'order-event' to frontend
 * 3. Listener calls _applyEvent
 * 4. Store updates, React re-renders
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import type {
  OrderSnapshot,
  OrderEvent,
  OrderConnectionState,
} from '@/core/domain/types/orderEvent';
import { applyEvent, createEmptySnapshot, verifyChecksum, computeChecksum } from './orderReducer';

// ============================================================================
// Store Interface
// ============================================================================

interface ActiveOrdersState {
  /** Map of order_id -> OrderSnapshot */
  orders: Map<string, OrderSnapshot>;
  /** Last processed event sequence number */
  lastSequence: number;
  /** Connection state for order sync */
  connectionState: OrderConnectionState;
  /** Whether the store has been initialized with server data */
  isInitialized: boolean;
  /**
   * Server instance epoch (UUID)
   * Used to detect server restarts - if epoch changes, full sync is required
   */
  serverEpoch: string | null;
}

interface ActiveOrdersActions {
  // ==================== Read-Only Queries ====================

  /**
   * Get order by ID
   */
  getOrder: (orderId: string) => OrderSnapshot | undefined;

  /**
   * Get all active orders (status === 'ACTIVE')
   */
  getActiveOrders: () => OrderSnapshot[];

  /**
   * Get order by table ID
   */
  getOrderByTable: (tableId: string) => OrderSnapshot | undefined;

  /**
   * Get all orders (including completed/voided)
   */
  getAllOrders: () => OrderSnapshot[];

  /**
   * Check if a table has an active order
   */
  hasActiveOrderOnTable: (tableId: string) => boolean;

  // ==================== Internal Methods (Event-Driven) ====================
  // These methods are prefixed with _ to indicate they should only be called
  // by event listeners, not by UI components directly.

  /**
   * Apply a single event to update state
   * Called when receiving 'order-event' from Tauri
   */
  _applyEvent: (event: OrderEvent) => void;

  /**
   * Apply multiple events in sequence
   * Called during reconnection sync
   */
  _applyEvents: (events: OrderEvent[]) => void;

  /**
   * Full sync: replace all orders with server state
   * Called when gap is too large, on initial load, or when server epoch changes
   *
   * @param orders - Active order snapshots from server
   * @param serverSequence - Server's current sequence number
   * @param serverEpoch - Server instance epoch (optional, updates if provided)
   */
  _fullSync: (orders: OrderSnapshot[], serverSequence: number, serverEpoch?: string) => void;

  /**
   * Update connection state
   * Called by connection status listener
   */
  _setConnectionState: (state: OrderConnectionState) => void;

  /**
   * Mark store as initialized
   */
  _setInitialized: (initialized: boolean) => void;

  /**
   * Reset store to initial state (for logout/tenant switch)
   */
  _reset: () => void;

  // ==================== Drift Detection ====================

  /**
   * Verify that all local snapshots have valid checksums
   * Returns list of order IDs with checksum mismatches
   * Called periodically or after applying multiple events
   */
  _detectLocalDrift: () => string[];

  /**
   * Compare local checksum with server-provided checksum
   * Returns true if checksums match, false if drift detected
   * Use this when receiving a snapshot from server to verify reducer consistency
   */
  _verifyServerChecksum: (orderId: string, serverChecksum: string) => boolean;

  /**
   * Get orders that need full sync due to checksum mismatch
   * Returns list of order IDs where local checksum differs from stored server checksum
   */
  _getDriftedOrders: () => string[];
}

type ActiveOrdersStore = ActiveOrdersState & ActiveOrdersActions;

// ============================================================================
// Store Implementation
// ============================================================================

const initialState: ActiveOrdersState = {
  orders: new Map(),
  lastSequence: 0,
  connectionState: 'disconnected',
  isInitialized: false,
  serverEpoch: null,
};

export const useActiveOrdersStore = create<ActiveOrdersStore>((set, get) => ({
  // Initial state
  ...initialState,

  // ==================== Read-Only Queries ====================

  getOrder: (orderId: string) => {
    return get().orders.get(orderId);
  },

  getActiveOrders: () => {
    const orders = get().orders;
    return Array.from(orders.values()).filter(
      (order) => order.status === 'ACTIVE'
    );
  },

  getOrderByTable: (tableId: string) => {
    const orders = get().orders;
    return Array.from(orders.values()).find(
      (order) => order.table_id === tableId && order.status === 'ACTIVE'
    );
  },

  getAllOrders: () => {
    return Array.from(get().orders.values());
  },

  hasActiveOrderOnTable: (tableId: string) => {
    const orders = get().orders;
    return Array.from(orders.values()).some(
      (order) => order.table_id === tableId && order.status === 'ACTIVE'
    );
  },

  // ==================== Internal Methods ====================

  _applyEvent: (event: OrderEvent) => {
    set((state) => {
      // Skip if event is older than our last sequence (duplicate)
      if (event.sequence <= state.lastSequence) {
        console.warn(
          `Skipping duplicate event: sequence ${event.sequence} <= ${state.lastSequence}`
        );
        return state;
      }

      // Detect sequence gap (missed events)
      if (event.sequence > state.lastSequence + 1) {
        console.warn(
          `Event sequence gap detected: expected ${state.lastSequence + 1}, got ${event.sequence}`
        );
        // The sync hook will handle reconnection
      }

      // Get or create the order snapshot
      const orders = new Map(state.orders);
      let snapshot = orders.get(event.order_id);

      if (!snapshot) {
        // New order - create empty snapshot
        snapshot = createEmptySnapshot(event.order_id);
      }

      // Apply the event
      const newSnapshot = applyEvent(snapshot, event);
      orders.set(event.order_id, newSnapshot);

      // If order is no longer active, we might want to remove it from memory
      // For now, keep all orders to support timeline views
      // Could add a cleanup mechanism later if memory becomes an issue

      return {
        ...state,
        orders,
        lastSequence: event.sequence,
      };
    });
  },

  _applyEvents: (events: OrderEvent[]) => {
    if (events.length === 0) return;

    set((state) => {
      // Sort events by sequence
      const sortedEvents = [...events].sort((a, b) => a.sequence - b.sequence);

      // Apply each event
      const orders = new Map(state.orders);
      let lastSequence = state.lastSequence;

      for (const event of sortedEvents) {
        // Skip duplicates
        if (event.sequence <= lastSequence) continue;

        let snapshot = orders.get(event.order_id);
        if (!snapshot) {
          snapshot = createEmptySnapshot(event.order_id);
        }

        const newSnapshot = applyEvent(snapshot, event);
        orders.set(event.order_id, newSnapshot);
        lastSequence = event.sequence;
      }

      return {
        ...state,
        orders,
        lastSequence,
      };
    });
  },

  _fullSync: (orders: OrderSnapshot[], serverSequence: number, serverEpoch?: string) => {
    set((state) => {
      const newOrders = new Map<string, OrderSnapshot>();

      for (const order of orders) {
        newOrders.set(order.order_id, order);
      }

      return {
        ...state,
        orders: newOrders,
        lastSequence: serverSequence,
        isInitialized: true,
        connectionState: 'connected',
        // Update epoch if provided
        serverEpoch: serverEpoch ?? state.serverEpoch,
      };
    });
  },

  _setConnectionState: (connectionState: OrderConnectionState) => {
    set({ connectionState });
  },

  _setInitialized: (isInitialized: boolean) => {
    set({ isInitialized });
  },

  _reset: () => {
    set(initialState);
  },

  // ==================== Drift Detection ====================

  _detectLocalDrift: () => {
    const orders = get().orders;
    const driftedOrders: string[] = [];

    for (const [orderId, snapshot] of orders) {
      // Compute current checksum and compare with stored
      const currentChecksum = computeChecksum(snapshot);
      if (snapshot.state_checksum && snapshot.state_checksum !== currentChecksum) {
        console.warn(
          `[DriftDetection] Local checksum mismatch for order ${orderId}: ` +
            `stored=${snapshot.state_checksum}, computed=${currentChecksum}`
        );
        driftedOrders.push(orderId);
      }
    }

    return driftedOrders;
  },

  _verifyServerChecksum: (orderId: string, serverChecksum: string) => {
    const snapshot = get().orders.get(orderId);
    if (!snapshot) {
      // Order doesn't exist locally - need sync
      return false;
    }

    const localChecksum = computeChecksum(snapshot);
    const match = localChecksum === serverChecksum;

    if (!match) {
      console.warn(
        `[DriftDetection] Server checksum mismatch for order ${orderId}: ` +
          `local=${localChecksum}, server=${serverChecksum}`
      );
    }

    return match;
  },

  _getDriftedOrders: () => {
    const orders = get().orders;
    const drifted: string[] = [];

    for (const [orderId, snapshot] of orders) {
      // If order has a server checksum but local computation differs
      if (snapshot.state_checksum) {
        const localChecksum = computeChecksum(snapshot);
        if (localChecksum !== snapshot.state_checksum) {
          drifted.push(orderId);
        }
      }
    }

    return drifted;
  },
}));

// ============================================================================
// Selectors (for optimized React re-renders)
// ============================================================================

/**
 * Select all active orders
 * Re-renders only when active orders change
 */
export const useActiveOrders = () =>
  useActiveOrdersStore((state) => state.getActiveOrders());

/**
 * Select a specific order by ID
 */
export const useOrder = (orderId: string | null | undefined) =>
  useActiveOrdersStore((state) =>
    orderId ? state.orders.get(orderId) : undefined
  );

/**
 * Select order by table ID
 */
export const useOrderByTable = (tableId: string | null | undefined) =>
  useActiveOrdersStore((state) => {
    if (!tableId) return undefined;
    return Array.from(state.orders.values()).find(
      (order) => order.table_id === tableId && order.status === 'ACTIVE'
    );
  });

/**
 * Select active order count
 */
export const useActiveOrderCount = () =>
  useActiveOrdersStore((state) =>
    Array.from(state.orders.values()).filter((o) => o.status === 'ACTIVE').length
  );

/**
 * Select connection state
 */
export const useOrderConnectionState = () =>
  useActiveOrdersStore((state) => state.connectionState);

/**
 * Select whether store is initialized
 */
export const useOrdersInitialized = () =>
  useActiveOrdersStore((state) => state.isInitialized);

/**
 * Select last sequence number
 */
export const useLastSequence = () =>
  useActiveOrdersStore((state) => state.lastSequence);

/**
 * Check if connected to order service
 */
export const useIsOrderConnected = () =>
  useActiveOrdersStore((state) => state.connectionState === 'connected');

/**
 * Select query methods for imperative use
 */
export const useOrderQueries = () =>
  useActiveOrdersStore(
    useShallow((state) => ({
      getOrder: state.getOrder,
      getActiveOrders: state.getActiveOrders,
      getOrderByTable: state.getOrderByTable,
      getAllOrders: state.getAllOrders,
      hasActiveOrderOnTable: state.hasActiveOrderOnTable,
    }))
  );

/**
 * Select internal methods (for event listeners only)
 * Use with caution - these should not be called from UI components
 */
export const useOrderStoreInternal = () =>
  useActiveOrdersStore(
    useShallow((state) => ({
      _applyEvent: state._applyEvent,
      _applyEvents: state._applyEvents,
      _fullSync: state._fullSync,
      _setConnectionState: state._setConnectionState,
      _setInitialized: state._setInitialized,
      _reset: state._reset,
    }))
  );
