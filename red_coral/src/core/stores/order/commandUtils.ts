/**
 * Order Command Utilities
 *
 * Shared utilities for creating and sending order commands.
 */

import { useBridgeStore } from '@/core/stores/bridge/useBridgeStore';
import type { OrderCommand, OrderCommandPayload } from '@/core/domain/types/orderEvent';

/**
 * Generate a temporary command ID (number).
 * Server will override with its own snowflake ID, so this only needs
 * to be unique enough for short-lived client-side dedup.
 */
function generateCommandId(): number {
  return Date.now();
}

/**
 * Create a command wrapper with operator info
 */
export function createCommand(payload: OrderCommandPayload): OrderCommand {
  const session = useBridgeStore.getState().currentSession;
  const operatorId = session?.user_info?.id ?? 0;
  const operatorName = session?.user_info?.name ?? 'Unknown';

  return {
    command_id: generateCommandId(),
    timestamp: Date.now(),
    operator_id: operatorId,
    operator_name: operatorName,
    payload,
  };
}
