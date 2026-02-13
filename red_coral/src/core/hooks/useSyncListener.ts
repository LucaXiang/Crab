/**
 * Sync Listener Hook - 服务器权威模型
 *
 * 监听 server-message 事件中的 Sync 信号，触发 Store 刷新。
 * 使用 resources/ 下的统一 Store 架构。
 *
 * 特殊处理:
 * - "lagged" 类型: WiFi 丢包恢复，触发 Order 全量重同步
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { logger } from '@/utils/logger';
import { storeRegistry } from '@/core/stores/resources/registry';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useShiftStore } from '@/core/stores/shift';
import type { SyncResponse } from '@/core/domain/types/orderEvent';
import type { Shift } from '@/core/domain/types/api';

const PRODUCT_REFRESH_DEBOUNCE_MS = 500;

interface SyncPayload {
  resource: string;
  action: 'created' | 'updated' | 'deleted' | 'settlement_required';
  id: string | number; // Backend sends String, parsed to number before applySync
  version: number;
  data: unknown | null;
}

/**
 * WiFi 丢包恢复消息 payload
 * 服务端在 broadcast channel lagged 时发送
 */
interface LaggedSyncPayload {
  reason: 'lagged';
  dropped_messages: number;
  action: 'full_resync';
}

interface ServerMessageEvent {
  event_type: string;
  payload: SyncPayload | LaggedSyncPayload | string;
  correlation_id: string | null;
}

/**
 * 检查是否为 lagged 重同步消息
 */
function isLaggedPayload(payload: unknown): payload is LaggedSyncPayload {
  if (typeof payload === 'object' && payload !== null) {
    const p = payload as Record<string, unknown>;
    return p.reason === 'lagged' && p.action === 'full_resync';
  }
  // 服务端发送的是 JSON 字符串，需要解析
  if (typeof payload === 'string') {
    try {
      const parsed = JSON.parse(payload);
      return parsed.reason === 'lagged' && parsed.action === 'full_resync';
    } catch {
      return false;
    }
  }
  return false;
}

/**
 * 解析 lagged payload
 */
function parseLaggedPayload(payload: unknown): LaggedSyncPayload | null {
  if (typeof payload === 'object' && payload !== null) {
    const p = payload as LaggedSyncPayload;
    if (p.reason === 'lagged') return p;
  }
  if (typeof payload === 'string') {
    try {
      const parsed = JSON.parse(payload);
      if (parsed.reason === 'lagged') return parsed as LaggedSyncPayload;
    } catch {
      return null;
    }
  }
  return null;
}

/**
 * 监听同步信号，触发 Store 刷新
 *
 * 服务器权威：收到 Sync 信号直接全量刷新对应资源
 */
/**
 * 处理 sequence gap 检测触发的全量同步
 */
async function handleGapDetectedSync(): Promise<void> {
  const { _fullSync, _setConnectionState, lastSequence } = useActiveOrdersStore.getState();

  logger.debug(`Gap detected recovery starting from lastSequence=${lastSequence}`, { component: 'Sync' });

  try {
    // 请求全量同步
    const response = await invokeApi<SyncResponse>('order_sync_since', {
      sinceSequence: 0,
    });

    if (response) {
      _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
      logger.debug(`Gap recovery complete - synced ${response.active_orders.length} orders, epoch=${response.server_epoch}`, { component: 'Sync' });
    }
  } catch (err) {
    logger.error('Gap recovery failed', err, { component: 'Sync' });
    _setConnectionState('disconnected');
  }
}

export function useSyncListener() {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let isMounted = true;
    let productRefreshTimer: ReturnType<typeof setTimeout> | null = null;

    // 监听 sequence gap 检测事件
    const handleGapEvent = () => {
      handleGapDetectedSync();
    };
    window.addEventListener('order-sync-gap-detected', handleGapEvent);

    listen<ServerMessageEvent>('server-message', async (event) => {
      const message = event.payload;

      // Only handle Sync type messages
      if (message.event_type !== 'sync') return;

      // 检查是否为 WiFi 丢包恢复消息
      if (isLaggedPayload(message.payload)) {
        const laggedInfo = parseLaggedPayload(message.payload);
        logger.warn(`WiFi lag detected - dropped ${laggedInfo?.dropped_messages || '?'} messages, triggering full resync`, { component: 'Sync' });

        // 触发 Order 全量重同步
        const { _fullSync, _setConnectionState } = useActiveOrdersStore.getState();
        _setConnectionState('syncing');

        try {
          // 请求从 sequence 0 开始的全量同步
          const response = await invokeApi<SyncResponse>('order_sync_since', {
            sinceSequence: 0,
          });

          if (response) {
            _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
            logger.debug(`WiFi lag recovery complete - synced ${response.active_orders.length} orders, epoch=${response.server_epoch}`, { component: 'Sync' });
          }
        } catch (err) {
          logger.error('WiFi lag recovery failed', err, { component: 'Sync' });
          _setConnectionState('disconnected');
        }

        return;
      }

      // 常规资源同步
      const payload = message.payload as SyncPayload;
      const { resource, id: rawId, version, action, data } = payload;
      // Backend SyncPayload.id is Rust String → JSON string "1",
      // but store items use numeric ids. Parse to number for correct === matching.
      const id = typeof rawId === 'number' ? rawId : Number(rawId);
      logger.debug(`Received sync event: resource=${resource}, action=${action}, id=${id}`, { component: 'SyncListener' });

      // 特殊处理: shift 事件
      if (resource === 'shift') {
        if (action === 'settlement_required') {
          logger.debug('Shift settlement required, notifying ShiftGuard', { component: 'SyncListener' });
          const shiftData = data as Shift | null;
          if (shiftData) {
            useShiftStore.getState().setStaleShift(shiftData);
          }
        } else if (action === 'updated' && data) {
          const shiftData = data as Shift;
          const { currentShift } = useShiftStore.getState();
          if (currentShift && currentShift.id === shiftData.id) {
            useShiftStore.setState({ currentShift: shiftData });
          }
        }
        return;
      }

      // 特殊处理: order_sync - 包含 event (时间线) + snapshot (状态)
      if (resource === 'order_sync') {
        if (data) {
          const { event, snapshot } = data as {
            event: import('@/core/domain/types/orderEvent').OrderEvent;
            snapshot: import('@/core/domain/types/orderEvent').OrderSnapshot;
          };
          logger.debug(`Order sync: ${event.event_type}, order=${snapshot.order_id}`, { component: 'SyncListener' });
          useActiveOrdersStore.getState()._applyOrderSync(event, snapshot);
        }
        return;
      }

      // 调用 resources store 的 applySync（只处理标准 CRUD 操作）
      const store = storeRegistry[resource];
      if (store && (action === 'created' || action === 'updated' || action === 'deleted')) {
        logger.debug(`Found store for ${resource}, calling applySync`, { component: 'SyncListener' });
        store.getState().applySync({ id, version, action, data });
      } else if (!store) {
        logger.warn(`No store found for resource: ${resource}`, { component: 'SyncListener' });
      }

      // 属性/分类变更时级联刷新 product store（ProductFull 嵌入了完整属性数据）
      // 防抖：批量变更时合并为一次刷新
      if ((resource === 'attribute' || resource === 'category') && storeRegistry.product) {
        if (productRefreshTimer) clearTimeout(productRefreshTimer);
        productRefreshTimer = setTimeout(() => {
          productRefreshTimer = null;
          const productStore = storeRegistry.product?.getState();
          if (productStore?.isLoaded) {
            logger.debug(`${resource} changed, refreshing product store (debounced)`, { component: 'SyncListener' });
            productStore.fetchAll(true);
          }
        }, PRODUCT_REFRESH_DEBOUNCE_MS);
      }
    }).then((fn) => {
      if (isMounted) {
        unlisten = fn;
      } else {
        // Already unmounted, clean up immediately
        fn();
      }
    });

    return () => {
      isMounted = false;
      unlisten?.();
      if (productRefreshTimer) clearTimeout(productRefreshTimer);
      window.removeEventListener('order-sync-gap-detected', handleGapEvent);
    };
  }, []);
}
