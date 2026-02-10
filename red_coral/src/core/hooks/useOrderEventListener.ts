/**
 * Order Event Listener Hook (Server Authority Model)
 *
 * Sets up Tauri event listeners for order sync and handles initialization.
 * This hook should be called once at the app level (App.tsx) after server mode starts.
 *
 * Server Authority Model:
 * - Client NEVER computes snapshots locally
 * - Backend sends 'order-sync' events containing BOTH event AND snapshot
 * - Store uses server-provided snapshots directly
 *
 * Event Flow:
 * 1. Listen for 'order-sync' events from Tauri backend
 * 2. Event payload contains { event, snapshot } - no API call needed
 * 3. Apply (event, snapshot) pair to store directly
 */

import { useEffect, useRef, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useShallow } from 'zustand/shallow';
import { invokeApi } from '@/infrastructure/api';
import { logger } from '@/utils/logger';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useBridgeStore } from '@/core/stores/bridge/useBridgeStore';
import type { OrderEvent, OrderSnapshot, SyncResponse } from '@/core/domain/types/orderEvent';

/** Payload structure for order-sync Tauri events (matches Rust OrderSyncPayload) */
interface OrderSyncPayload {
  event: OrderEvent;
  snapshot: OrderSnapshot;
}

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
        sinceSequence: 0,
      });

      // Full sync with server state (including events for timeline and server_epoch)
      store._fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
      store._setInitialized(true);
      isInitializedRef.current = true;

      logger.debug(`Initialized with ${response.active_orders.length} active orders, sequence: ${response.server_sequence}`, { component: 'OrderEventListener' });
    } catch (error) {
      logger.error('Failed to initialize orders', error, { component: 'OrderEventListener' });
      store._setConnectionState('disconnected');
    }
  }, []);

  // Set up event listener (Server Authority Model)
  const setupListener = useCallback(async () => {
    // Clean up existing listener
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }

    // Listen for order-sync events from Tauri (contains event + snapshot)
    // Server Authority: backend sends snapshot directly, no API call needed
    const unlisten = await listen<OrderSyncPayload>('order-sync', (event) => {
      const { event: orderEvent, snapshot } = event.payload;
      logger.debug(`Received sync: ${orderEvent.event_type} for order ${orderEvent.order_id}`, { component: 'OrderEventListener' });

      // Apply with server-computed snapshot directly (no API call)
      useActiveOrdersStore.getState()._applyOrderSync(orderEvent, snapshot);
    });

    unlistenRef.current = unlisten;
    logger.debug('Event listener set up (Server Authority Mode)', { component: 'OrderEventListener' });
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
    if (appState?.type === 'NeedTenantLogin' || appState?.type === 'TenantReady') {
      isInitializedRef.current = false;
      useActiveOrdersStore.getState()._reset();
    }
  }, [appState?.type]);
}

/**
 * Hook to get order sync utilities (Server Authority Model)
 *
 * Provides methods for manual sync operations (reconnection, refresh, etc.)
 * Always uses full sync - client never computes snapshots locally.
 */
export function useOrderSyncActions() {
  // Server Authority: always perform full sync (since_sequence = 0)
  const syncOrders = useCallback(async () => {
    const store = useActiveOrdersStore.getState();
    store._setConnectionState('syncing');

    try {
      const response = await invokeApi<SyncResponse>('order_sync_since', {
        sinceSequence: 0, // Always full sync (Server Authority Model)
      });

      store._fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);

      return true;
    } catch (error) {
      logger.error('Sync failed', error, { component: 'OrderSync' });
      store._setConnectionState('disconnected');
      return false;
    }
  }, []);

  const refreshOrders = useCallback(async () => {
    return syncOrders();
  }, [syncOrders]);

  return {
    syncOrders,
    refreshOrders,
  };
}

/** Response type for order events API */
interface OrderEventsResponse {
  events: OrderEvent[];
}

/** Max retry attempts for timeline sync */
const MAX_SYNC_RETRIES = 3;

/** Base delay for retry backoff (ms) */
const RETRY_BASE_DELAY = 1000;

/**
 * Hook to automatically sync timelines when sequence gaps are detected
 *
 * This hook watches for orders that need timeline sync (due to sequence gaps)
 * and automatically fetches their events from the server.
 *
 * Features:
 * - Deduplication: prevents concurrent syncs for the same order
 * - Retry with exponential backoff on failure
 * - Cleanup on unmount
 *
 * Usage in App.tsx (after useOrderEventListener):
 * ```tsx
 * const App = () => {
 *   useOrderEventListener();
 *   useOrderTimelineSync(); // Auto-sync timelines on gap
 *   // ...
 * };
 * ```
 */
export function useOrderTimelineSync() {
  const syncingRef = useRef<Set<string>>(new Set());
  const retryCountRef = useRef<Map<string, number>>(new Map());
  const mountedRef = useRef(true);

  // 使用 useShallow 防止 Array.from 创建的新数组导致无限重渲染
  const ordersNeedingSync = useActiveOrdersStore(
    useShallow((state) => Array.from(state.ordersNeedingTimelineSync))
  );

  useEffect(() => {
    mountedRef.current = true;

    const syncTimeline = async (orderId: string) => {
      // 防止重复同步
      if (syncingRef.current.has(orderId)) return;
      syncingRef.current.add(orderId);

      logger.debug(`Fetching events for order ${orderId}`, { component: 'TimelineSync' });

      try {
        const response = await invokeApi<OrderEventsResponse>(
          'order_get_events_for_order',
          { orderId: orderId }
        );

        // 检查组件是否仍然挂载
        if (!mountedRef.current) return;

        useActiveOrdersStore.getState()._syncOrderTimeline(orderId, response.events);
        retryCountRef.current.delete(orderId); // 成功后清除重试计数
        logger.debug(`Synced ${response.events.length} events for order ${orderId}`, { component: 'TimelineSync' });
      } catch (error) {
        logger.error(`Failed to sync order ${orderId}`, error, { component: 'TimelineSync' });

        if (!mountedRef.current) return;

        // 重试逻辑
        const retries = retryCountRef.current.get(orderId) || 0;
        if (retries < MAX_SYNC_RETRIES) {
          retryCountRef.current.set(orderId, retries + 1);
          const delay = RETRY_BASE_DELAY * Math.pow(2, retries);
          logger.debug(`Retry ${retries + 1}/${MAX_SYNC_RETRIES} for ${orderId} in ${delay}ms`, { component: 'TimelineSync' });
          
          setTimeout(() => {
            if (mountedRef.current) {
              syncingRef.current.delete(orderId); // 允许重新同步
              syncTimeline(orderId);
            }
          }, delay);
        } else {
          logger.error(`Max retries exceeded for ${orderId}`, undefined, { component: 'TimelineSync' });
          useActiveOrdersStore.getState()._clearTimelineSyncRequest(orderId);
          retryCountRef.current.delete(orderId);
        }
      } finally {
        if (retryCountRef.current.get(orderId) === undefined) {
          // 只有在不重试时才清除 syncing 标记
          syncingRef.current.delete(orderId);
        }
      }
    };

    // 同步所有需要补全的订单
    ordersNeedingSync.forEach((orderId) => {
      syncTimeline(orderId);
    });

    return () => {
      mountedRef.current = false;
    };
  }, [ordersNeedingSync]);
}
