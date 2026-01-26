/**
 * Order Command Utilities
 *
 * Shared utilities for creating and sending order commands.
 */

import { useBridgeStore } from '@/core/stores/bridge/useBridgeStore';
import type { OrderCommand, OrderCommandPayload } from '@/core/domain/types/orderEvent';

/**
 * Generate a UUID v4 for command idempotency
 */
export function generateCommandId(): string {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  // Fallback UUID generation
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * Create a command wrapper with operator info
 */
export function createCommand(payload: OrderCommandPayload): OrderCommand {
  const session = useBridgeStore.getState().currentSession;
  const operatorId = session?.user_info?.id ?? 'unknown';
  const operatorName = session?.user_info?.display_name ?? 'Unknown';

  return {
    command_id: generateCommandId(),
    timestamp: Date.now(),
    operator_id: operatorId,
    operator_name: operatorName,
    payload,
  };
}
