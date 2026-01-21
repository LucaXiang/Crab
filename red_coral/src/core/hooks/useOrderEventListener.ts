/**
 * Order Event Listener Hook
 *
 * Sets up Tauri event listeners for order events and handles initialization.
 * This hook should be called once at the app level (App.tsx) after server mode starts.
 *
 * Responsibilities:
 * 1. Listen for 'order-event' events from Tauri backend
 * 2. Apply events to useActiveOrdersStore
 * 3. Initialize order state on first connection
 * 4. Handle reconnection scenarios
 */

import { useEffect, useRef, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invokeApi } from '@/infrastructure/api';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useBridgeStore } from '@/core/stores/bridge/useBridgeStore';
import type { OrderEvent, OrderSnapshot, SyncResponse } from '@/core/domain/types/orderEvent';

/**
 * Hook to set up order event listeners and initialize order state
 *
 * Usage in App.tsx:
 * ```tsx
 * const App = () => {
 *   useOrderEventListener();
 *   // ...
 * };
 * ```
 */
export function useOrderEventListener() {
  const isInitializedRef = useRef(false);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const appState = useBridgeStore((state) => state.appState);

  // Initialize order state from server
  const initializeOrders = useCallback(async () => {
    if (isInitializedRef.current) return;

    const store = useActiveOrdersStore.getState();
    store._setConnectionState('syncing');

    try {
      // Fetch all active orders from server
      const response = await invokeApi<SyncResponse>('order_sync_since', {
        since_sequence: 0,
      });

      // Full sync with server state
      store._fullSync(response.active_orders, response.server_sequence);
      store._setInitialized(true);
      isInitializedRef.current = true;

      console.log(
        `[OrderEventListener] Initialized with ${response.active_orders.length} active orders, sequence: ${response.server_sequence}`
      );
    } catch (error) {
      console.error('[OrderEventListener] Failed to initialize orders:', error);
      store._setConnectionState('disconnected');
    }
  }, []);

  // Set up event listener
  const setupListener = useCallback(async () => {
    // Clean up existing listener
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }

    // Listen for order events from Tauri
    const unlisten = await listen<OrderEvent>('order-event', (event) => {
      const orderEvent = event.payload;
      console.log(
        `[OrderEventListener] Received event: ${orderEvent.event_type} for order ${orderEvent.order_id}`
      );
      useActiveOrdersStore.getState()._applyEvent(orderEvent);
    });

    unlistenRef.current = unlisten;
    console.log('[OrderEventListener] Event listener set up');
  }, []);

  useEffect(() => {
    // Set up listeners when in authenticated state (Server or Client mode)
    const shouldListen =
      appState?.type === 'ServerAuthenticated' ||
      appState?.type === 'ServerReady' ||
      appState?.type === 'ClientAuthenticated';

    if (shouldListen) {
      setupListener();
      initializeOrders();
    }

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [appState?.type, setupListener, initializeOrders]);

  // Reset on logout/disconnect
  useEffect(() => {
    if (appState?.type === 'Uninitialized' || appState?.type === 'ServerNoTenant') {
      isInitializedRef.current = false;
      useActiveOrdersStore.getState()._reset();
    }
  }, [appState?.type]);
}

/**
 * Hook to get order sync utilities
 *
 * Provides methods for manual sync operations (reconnection, refresh, etc.)
 */
export function useOrderSyncActions() {
  const syncOrders = useCallback(async (since_sequence: number = 0) => {
    const store = useActiveOrdersStore.getState();
    store._setConnectionState('syncing');

    try {
      const response = await invokeApi<SyncResponse>('order_sync_since', {
        since_sequence,
      });

      if (response.requires_full_sync || since_sequence === 0) {
        store._fullSync(response.active_orders, response.server_sequence);
      } else if (response.events.length > 0) {
        store._applyEvents(response.events);
        store._setConnectionState('connected');
      } else {
        store._setConnectionState('connected');
      }

      return true;
    } catch (error) {
      console.error('[OrderSync] Sync failed:', error);
      store._setConnectionState('disconnected');
      return false;
    }
  }, []);

  const refreshOrders = useCallback(async () => {
    return syncOrders(0);
  }, [syncOrders]);

  return {
    syncOrders,
    refreshOrders,
  };
}
