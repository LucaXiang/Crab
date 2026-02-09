/**
 * Unified command sending layer with command lock check.
 *
 * - Checks command lock before execution (prevents dirty writes during sync)
 * - ensureSuccess throws on failure for clean caller error handling
 */

import { invokeApi } from '@/infrastructure/api/tauri-client';
import { checkCommandLock } from '@/core/hooks/useCommandLock';
import { logger } from '@/utils/logger';
import type { OrderCommand, CommandResponse } from '@/core/domain/types/orderEvent';

/**
 * Send an order command to the backend.
 *
 * Checks command lock before execution — if the system is syncing
 * or disconnected, returns an error immediately to prevent dirty data.
 */
export async function sendCommand(command: OrderCommand): Promise<CommandResponse> {
  const lockCheck = checkCommandLock();
  if (!lockCheck.canExecute) {
    logger.warn(`Command blocked: ${command.payload.type} - ${lockCheck.error}`, { component: 'OrderCommands' });
    return {
      command_id: command.command_id,
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: lockCheck.error ?? 'System is busy, please wait',
      },
    };
  }

  try {
    return await invokeApi<CommandResponse>('order_execute_command', { command });
  } catch (error: unknown) {
    logger.error('Command failed', error);
    return {
      command_id: command.command_id,
      success: false,
      error: {
        code: 'INTERNAL_ERROR',
        message: error instanceof Error ? error.message : 'Command execution failed',
      },
    };
  }
}

/**
 * Assert that a command response indicates success, throws on failure.
 */
export function ensureSuccess(response: CommandResponse, context: string): void {
  if (!response.success) {
    const message = response.error?.message || `${context} failed`;
    logger.error(`OrderOps ${context}: ${message}`);
    throw new Error(message);
  }
}
