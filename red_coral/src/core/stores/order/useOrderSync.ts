/**
 * Order Sync Hook (Server Authority Model)
 *
 * Handles reconnection and synchronization of order state after network disconnection.
 *
 * Server Authority Model:
 * - Client NEVER computes snapshots locally
 * - All sync operations use server-provided snapshots via _fullSync
 * - No incremental event application (no local computation)
 *
 * Sync Protocol:
 * 1. On disconnect: Mark connection state as 'disconnected'
 * 2. On reconnect: Request full sync from server
 * 3. Server returns: active orders (snapshots) + events (for timeline)
 * 4. Always use _fullSync to replace state with server data
 *
 * Usage:
 * const { syncOrders, reconnect, isReconnecting } = useOrderSync();
 *
 * // On network recovery
 * await reconnect();
 */

import { useCallback, useState } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useActiveOrdersStore } from './useActiveOrdersStore';
import type { SyncResponse, OrderEvent, OrderSnapshot } from '@/core/domain/types/orderEvent';

// ============================================================================
// Constants
// ============================================================================

/** Maximum event gap before requiring full sync */
const MAX_EVENT_GAP = 1000;

/** Exponential backoff configuration */
const BACKOFF_BASE_DELAY = 1000;     // Start with 1 second
const BACKOFF_MULTIPLIER = 1.5;      // Multiply by 1.5 each attempt
const BACKOFF_MAX_DELAY = 30000;     // Cap at 30 seconds
const BACKOFF_JITTER = 0.1;          // Add 10% random jitter

/** Maximum reconnection attempts */
const MAX_RECONNECT_ATTEMPTS = 10;   // Increased due to exponential backoff

/**
 * Calculate exponential backoff delay with jitter
 * @param attempt - Current attempt number (0-indexed)
 * @returns Delay in milliseconds
 */
function calculateBackoffDelay(attempt: number): number {
  // Calculate base exponential delay: base * multiplier^attempt
  const exponentialDelay = BACKOFF_BASE_DELAY * Math.pow(BACKOFF_MULTIPLIER, attempt);

  // Cap at max delay
  const cappedDelay = Math.min(exponentialDelay, BACKOFF_MAX_DELAY);

  // Add jitter (±10%) to prevent thundering herd
  const jitter = cappedDelay * BACKOFF_JITTER * (Math.random() * 2 - 1);

  return Math.round(cappedDelay + jitter);
}

// ============================================================================
// Hook Implementation
// ============================================================================

export function useOrderSync() {
  const [isReconnecting, setIsReconnecting] = useState(false);
  const [reconnectAttempts, setReconnectAttempts] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const store = useActiveOrdersStore.getState();

  /**
   * Request sync from server
   */
  const syncOrders = useCallback(async (sinceSequence: number): Promise<SyncResponse | null> => {
    try {
      const response = await invokeApi<SyncResponse>('order_sync_since', {
        sinceSequence,
      });
      return response;
    } catch (err: unknown) {
      console.error('Failed to sync orders:', err);
      setError(err instanceof Error ? err.message : 'Sync failed');
      return null;
    }
  }, []);

  /**
   * Perform reconnection and sync (Server Authority Model)
   *
   * Always performs full sync - client never computes snapshots locally.
   * Server provides both snapshots and events (for timeline display).
   */
  const reconnect = useCallback(async (): Promise<boolean> => {
    const { _fullSync, _setConnectionState } =
      useActiveOrdersStore.getState();

    setIsReconnecting(true);
    _setConnectionState('syncing');
    setError(null);

    try {
      // Always request full sync from sequence 0 (Server Authority Model)
      // This ensures we get server-computed snapshots, not incremental events
      const response = await syncOrders(0);

      if (!response) {
        throw new Error('Failed to get sync response');
      }

      // Server Authority: always use full sync with server-provided snapshots
      console.log(`[Sync] Full sync: ${response.active_orders.length} orders, epoch=${response.server_epoch}`);
      _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);

      setReconnectAttempts(0);
      setIsReconnecting(false);
      return true;
    } catch (err: unknown) {
      console.error('[Sync] Reconnection failed:', err);
      setError(err instanceof Error ? err.message : 'Reconnection failed');
      _setConnectionState('disconnected');
      setIsReconnecting(false);

      // Increment attempt counter
      setReconnectAttempts((prev) => prev + 1);
      return false;
    }
  }, [syncOrders]);

  /**
   * Attempt reconnection with exponential backoff retry logic
   *
   * Backoff sequence (with jitter): ~1s, ~1.5s, ~2.25s, ~3.4s, ~5s, ~7.6s, ~11s, ~17s, ~26s, ~30s (capped)
   */
  const reconnectWithRetry = useCallback(async (): Promise<boolean> => {
    let attempts = 0;

    while (attempts < MAX_RECONNECT_ATTEMPTS) {
      const success = await reconnect();
      if (success) return true;

      attempts++;
      if (attempts < MAX_RECONNECT_ATTEMPTS) {
        // Calculate exponential backoff delay
        const delay = calculateBackoffDelay(attempts - 1);
        console.log(`[Sync] Reconnect attempt ${attempts}/${MAX_RECONNECT_ATTEMPTS} failed, retrying in ${delay}ms`);
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }

    console.error(`[Sync] Failed to reconnect after ${MAX_RECONNECT_ATTEMPTS} attempts`);
    return false;
  }, [reconnect]);

  /**
   * Initialize order state from server (called on app startup)
   */
  const initializeFromServer = useCallback(async (): Promise<boolean> => {
    const { _fullSync, _setConnectionState, _setInitialized } =
      useActiveOrdersStore.getState();

    _setConnectionState('syncing');

    try {
      // Request full sync from sequence 0
      const response = await syncOrders(0);

      if (!response) {
        throw new Error('Failed to get initial sync response');
      }

      // Full sync with all active orders, storing the server epoch
      console.log(`[Sync] Initial sync: ${response.active_orders.length} orders, epoch=${response.server_epoch}`);
      _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
      _setInitialized(true);

      return true;
    } catch (err: unknown) {
      console.error('[Sync] Failed to initialize from server:', err);
      setError(err instanceof Error ? err.message : 'Initialization failed');
      _setConnectionState('disconnected');
      return false;
    }
  }, [syncOrders]);

  /**
   * Reset sync state (for logout/tenant switch)
   */
  const reset = useCallback(() => {
    setIsReconnecting(false);
    setReconnectAttempts(0);
    setError(null);
    useActiveOrdersStore.getState()._reset();
  }, []);

  return {
    // State
    isReconnecting,
    reconnectAttempts,
    error,

    // Actions
    syncOrders,
    reconnect,
    reconnectWithRetry,
    initializeFromServer,
    reset,

    // Constants (for UI display)
    maxAttempts: MAX_RECONNECT_ATTEMPTS,
  };
}

/** Payload structure for order-sync Tauri events (matches Rust OrderSyncPayload) */
interface OrderSyncPayload {
  event: OrderEvent;
  snapshot: OrderSnapshot;
}

// ============================================================================
// Event Listener Setup (Server Authority Model)
// ============================================================================

/**
 * Setup Tauri event listeners for order sync and connection status
 *
 * Server Authority Model:
 * - Backend sends 'order-sync' events containing BOTH event AND snapshot
 * - No API calls needed - snapshot is provided directly
 * - On sync request, always perform full sync
 *
 * Call this once in App.tsx or a provider component:
 *
 * useEffect(() => {
 *   const cleanup = setupOrderEventListeners();
 *   return cleanup;
 * }, []);
 */
export async function setupOrderEventListeners(): Promise<() => void> {
  const unlistenFns: UnlistenFn[] = [];

  // Listen for order-sync events (Server Authority: event + snapshot bundled)
  const unlistenOrderSync = await listen<OrderSyncPayload>('order-sync', (event) => {
    const { event: orderEvent, snapshot } = event.payload;
    console.log(`[OrderSync] Received sync: ${orderEvent.event_type} for order ${orderEvent.order_id}`);

    // Apply with server-computed snapshot directly (no API call)
    useActiveOrdersStore.getState()._applyOrderSync(orderEvent, snapshot);
  });
  unlistenFns.push(unlistenOrderSync);

  // Listen for connection status changes
  const unlistenConnectionStatus = await listen<'connected' | 'disconnected'>(
    'order-connection',
    (event) => {
      const status = event.payload;
      useActiveOrdersStore.getState()._setConnectionState(status);

      // Auto-reconnect on disconnect
      if (status === 'disconnected') {
        console.log('Order connection lost, will attempt reconnect...');
      }
    }
  );
  unlistenFns.push(unlistenConnectionStatus);

  // Listen for sync requests (server can request client to resync)
  // Server Authority: always perform full sync
  const unlistenSyncRequest = await listen<{ since_sequence: number }>(
    'order-sync-request',
    async () => {
      console.log('[Sync] Server requested sync');
      const { _fullSync, _setConnectionState } = useActiveOrdersStore.getState();

      _setConnectionState('syncing');

      try {
        // Always full sync (Server Authority Model)
        const response = await invokeApi<SyncResponse>('order_sync_since', {
          sinceSequence: 0,
        });

        _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
      } catch (err) {
        console.error('[Sync] Sync request failed:', err);
        _setConnectionState('disconnected');
      }
    }
  );
  unlistenFns.push(unlistenSyncRequest);

  // Return cleanup function
  return () => {
    unlistenFns.forEach((unlisten) => unlisten());
  };
}

// ============================================================================
// Type Exports
// ============================================================================

export type OrderSyncHook = ReturnType<typeof useOrderSync>;
