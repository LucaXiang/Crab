/**
 * Command Lock Hook - 命令执行锁定
 *
 * 在系统同步期间阻止命令执行，防止产生脏数据。
 *
 * 使用场景:
 * - 重连同步时阻止新的订单操作
 * - 服务器重启检测到 epoch 变化时阻止操作
 * - 全量同步期间阻止操作
 *
 * @example
 * ```tsx
 * function AddItemButton() {
 *   const { canExecute, withLock } = useCommandLock();
 *
 *   const handleAdd = async () => {
 *     try {
 *       await withLock(() => addItem(...));
 *     } catch (e) {
 *       // 显示 "系统同步中，请稍候"
 *       toast.error(e.message);
 *     }
 *   };
 *
 *   return (
 *     <Button disabled={!canExecute} onClick={handleAdd}>
 *       Add Item
 *     </Button>
 *   );
 * }
 * ```
 */

import { useCallback } from 'react';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { t } from '@/infrastructure/i18n';

/**
 * Command lock error thrown when trying to execute during sync
 */
class CommandLockError extends Error {
  constructor(
    message: string = 'System is syncing, please wait...',
    public readonly connectionState: string
  ) {
    super(message);
    this.name = 'CommandLockError';
  }
}

/**
 * Hook to check and enforce command locking during sync
 */
function useCommandLock() {
  const connectionState = useActiveOrdersStore((s) => s.connectionState);
  const isInitialized = useActiveOrdersStore((s) => s.isInitialized);

  /**
   * Whether commands can be executed
   * Only true when connected AND initialized
   */
  const canExecute = connectionState === 'connected' && isInitialized;

  /**
   * Whether the system is actively syncing
   */
  const isSyncing = connectionState === 'syncing';

  /**
   * Whether the system is disconnected
   */
  const isDisconnected = connectionState === 'disconnected';

  /**
   * Get human-readable status message for UI
   */
  const statusMessage = (() => {
    if (isSyncing) return t('system.syncing');
    if (isDisconnected) return t('system.disconnected');
    if (!isInitialized) return t('system.initializing');
    return null;
  })();

  /**
   * Wrap an async function with command lock check
   * Throws CommandLockError if cannot execute
   */
  const withLock = useCallback(
    async <T>(fn: () => Promise<T>): Promise<T> => {
      // Re-check state at execution time (may have changed)
      const { connectionState: currentState, isInitialized: currentInit } =
        useActiveOrdersStore.getState();

      if (currentState !== 'connected' || !currentInit) {
        const message =
          currentState === 'syncing'
            ? t('system.syncing_wait')
            : currentState === 'disconnected'
              ? t('system.disconnected_reconnecting')
              : t('system.initializing_wait');

        throw new CommandLockError(message, currentState);
      }

      return fn();
    },
    []
  );

  /**
   * Synchronous check - useful for quick UI state decisions
   */
  const assertCanExecute = useCallback(() => {
    const { connectionState: currentState, isInitialized: currentInit } =
      useActiveOrdersStore.getState();

    if (currentState !== 'connected' || !currentInit) {
      const message =
        currentState === 'syncing'
          ? t('system.syncing_wait')
          : currentState === 'disconnected'
            ? t('system.disconnected_reconnecting')
            : t('system.initializing_wait');

      throw new CommandLockError(message, currentState);
    }
  }, []);

  return {
    /** Whether commands can be executed right now */
    canExecute,
    /** Whether the system is actively syncing */
    isSyncing,
    /** Whether the system is disconnected */
    isDisconnected,
    /** Human-readable status message (null if connected) */
    statusMessage,
    /** Wrap an async function with lock check */
    withLock,
    /** Synchronous assertion that throws if cannot execute */
    assertCanExecute,
  };
}

/**
 * Non-hook version for use outside React components
 * Useful for Zustand actions or utility functions
 */
export function checkCommandLock(): { canExecute: boolean; error: string | null } {
  const { connectionState, isInitialized } = useActiveOrdersStore.getState();

  if (connectionState === 'connected' && isInitialized) {
    return { canExecute: true, error: null };
  }

  const error =
    connectionState === 'syncing'
      ? t('system.syncing_wait')
      : connectionState === 'disconnected'
        ? t('system.disconnected_reconnecting')
        : t('system.initializing_wait');

  return { canExecute: false, error };
}
