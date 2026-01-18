/**
 * Server Message Hook for Tauri Events
 *
 * This hook provides a React-friendly way to listen for server messages
 * (Notification, Sync, ServerCommand) forwarded from the Rust backend
 * via Tauri Events.
 */

import { useEffect, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

/**
 * Server message structure as sent from Rust backend
 */
export interface ServerMessage {
  /** Event type (e.g., "notification", "sync", "server_command") */
  event_type: string;
  /** Payload data (type depends on event_type) */
  payload: unknown;
  /** Correlation ID for RPC response matching (optional) */
  correlation_id?: string;
}

/**
 * Handler function type for server messages
 */
export type MessageHandler = (message: ServerMessage) => void;

/**
 * Hook to listen for server messages via Tauri Events
 *
 * @param handler - Callback function to handle incoming server messages
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   useServerMessages((message) => {
 *     if (message.event_type === 'notification') {
 *       console.log('Notification:', message.payload);
 *     }
 *   });
 *
 *   return <div>Listening for server messages...</div>;
 * }
 * ```
 */
export function useServerMessages(handler: MessageHandler) {
  const stableHandler = useCallback(handler, [handler]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<ServerMessage>('server-message', (event) => {
      stableHandler(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [stableHandler]);
}
