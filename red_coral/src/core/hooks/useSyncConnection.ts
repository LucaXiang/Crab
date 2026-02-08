/**
 * Sync Connection Hook - 断线重连管理
 *
 * 监听连接状态变化，重连时检查 epoch 和 version，
 * 按需刷新过期的 Store。
 */

import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invokeApi } from '@/infrastructure/api';
import { logger } from '@/utils/logger';
import { storeRegistry, getLoadedStores, refreshAllLoadedStores } from '@/core/stores/resources';

interface SyncStatus {
  epoch: string;
  versions: Record<string, number>;
}

// 缓存的服务器 epoch
let cachedEpoch: string | null = null;

/**
 * 获取缓存的 epoch (用于测试)
 */
export function getCachedEpoch(): string | null {
  return cachedEpoch;
}

/**
 * 设置缓存的 epoch (用于初始化和测试)
 */
export function setCachedEpoch(epoch: string | null): void {
  cachedEpoch = epoch;
}

export function useSyncConnection() {
  const isReconnecting = useRef(false);

  useEffect(() => {
    const handleConnectionChange = async (connected: boolean) => {
      if (!connected || isReconnecting.current) return;

      isReconnecting.current = true;
      logger.debug('Reconnected, checking sync status', { component: 'SyncConnection' });

      try {
        const status = await invokeApi<SyncStatus>('get_sync_status');

        // Epoch 检查：epoch 变化说明服务器重启，需要全量刷新
        if (cachedEpoch && cachedEpoch !== status.epoch) {
          logger.warn('Epoch changed, full refresh all stores', { component: 'SyncConnection' });
          cachedEpoch = status.epoch;
          await refreshAllLoadedStores();
          return;
        }

        cachedEpoch = status.epoch;

        // Version 比对：只刷新落后的 Store
        const loadedStores = getLoadedStores();
        const staleStores: string[] = [];

        for (const [name, store] of loadedStores) {
          const serverVersion = status.versions[name] || 0;
          const checkVersion = store.getState().checkVersion;
          if (checkVersion && checkVersion(serverVersion)) {
            staleStores.push(name);
          }
        }

        if (staleStores.length > 0) {
          logger.debug(`Refreshing stale stores: ${staleStores.join(', ')}`, { component: 'SyncConnection' });
          await Promise.all(
            staleStores.map(name => storeRegistry[name].getState().fetchAll(true))
          );
        } else {
          logger.debug('All stores up to date', { component: 'SyncConnection' });
        }
      } catch (err) {
        logger.error('Sync status check failed, fallback to full refresh', err, { component: 'SyncConnection' });
        await refreshAllLoadedStores();
      } finally {
        isReconnecting.current = false;
      }
    };

    // 监听 Tauri 连接状态事件
    const unlisten = listen<boolean>('connection-state-changed', (event) => {
      handleConnectionChange(event.payload);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);
}
