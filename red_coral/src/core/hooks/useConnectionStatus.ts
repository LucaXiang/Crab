/**
 * Connection Status Hook for Tauri Events
 *
 * This hook provides a React-friendly way to listen for connection status
 * changes emitted by the backend connection monitor via Tauri Events.
 *
 * Integration Pattern:
 * - This hook (`useConnectionStatus`) is for components that need direct event listening
 *   with local state management (no global store overhead).
 * - The bridge store (`useBridgeStore`) exposes `setConnectionStatus` for updating global state
 *   and `useBridgeConnectionStatus` selector for reading it.
 * - A top-level component (like App.tsx or a ConnectionStatusProvider) can bridge them:
 *   use this hook to listen for events and call `setConnectionStatus` to sync to global state.
 */

import { useEffect, useState } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

/**
 * Connection status structure as sent from Rust backend
 */
export interface ConnectionStatus {
  /** Whether the connection to the edge server is active */
  connected: boolean;
  /** Whether the system is currently attempting to reconnect */
  reconnecting: boolean;
}

/**
 * Hook to track connection status via Tauri Events
 *
 * @returns Current connection status
 *
 * @example
 * ```tsx
 * function ConnectionIndicator() {
 *   const { connected, reconnecting } = useConnectionStatus();
 *
 *   if (reconnecting) {
 *     return <span>Reconnecting...</span>;
 *   }
 *
 *   return <span>{connected ? 'Online' : 'Offline'}</span>;
 * }
 * ```
 */
export function useConnectionStatus() {
  const [status, setStatus] = useState<ConnectionStatus>({
    connected: true,
    reconnecting: false,
  });

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<ConnectionStatus>('connection-status', (event) => {
      setStatus(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return status;
}
