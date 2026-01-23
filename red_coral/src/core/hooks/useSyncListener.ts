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
import { storeRegistry } from '@/core/stores/resources/registry';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import type { SyncResponse } from '@/core/domain/types/orderEvent';

interface SyncPayload {
  resource: string;
  action: 'created' | 'updated' | 'deleted';
  id: string;
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

  console.log(`[Sync] Gap detected recovery starting from lastSequence=${lastSequence}`);

  try {
    // 请求全量同步
    const response = await invokeApi<SyncResponse>('order_sync_since', {
      since_sequence: 0,
    });

    if (response) {
      _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
      console.log(
        `[Sync] Gap recovery complete - synced ${response.active_orders.length} orders, epoch=${response.server_epoch}`
      );
    }
  } catch (err) {
    console.error('[Sync] Gap recovery failed:', err);
    _setConnectionState('disconnected');
  }
}

export function useSyncListener() {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let isMounted = true;

    // 监听 sequence gap 检测事件
    const handleGapEvent = () => {
      handleGapDetectedSync();
    };
    window.addEventListener('order-sync-gap-detected', handleGapEvent);

    listen<ServerMessageEvent>('server-message', async (event) => {
      const message = event.payload;

      // Only handle Sync type messages (case-insensitive)
      if (message.event_type.toLowerCase() !== 'sync') return;

      // 检查是否为 WiFi 丢包恢复消息
      if (isLaggedPayload(message.payload)) {
        const laggedInfo = parseLaggedPayload(message.payload);
        console.warn(
          `[Sync] WiFi lag detected - dropped ${laggedInfo?.dropped_messages || '?'} messages, triggering full resync`
        );

        // 触发 Order 全量重同步
        const { _fullSync, _setConnectionState } = useActiveOrdersStore.getState();
        _setConnectionState('syncing');

        try {
          // 请求从 sequence 0 开始的全量同步
          const response = await invokeApi<SyncResponse>('order_sync_since', {
            since_sequence: 0,
          });

          if (response) {
            _fullSync(response.active_orders, response.server_sequence, response.server_epoch, response.events);
            console.log(
              `[Sync] WiFi lag recovery complete - synced ${response.active_orders.length} orders, epoch=${response.server_epoch}`
            );
          }
        } catch (err) {
          console.error('[Sync] WiFi lag recovery failed:', err);
          _setConnectionState('disconnected');
        }

        return;
      }

      // 常规资源同步
      const payload = message.payload as SyncPayload;
      const { resource, id, version, action, data } = payload;
      console.log(`[SyncListener] Received sync event: resource=${resource}, id=${id}, version=${version}`);

      // 特殊处理: order_sync - 包含 event (时间线) + snapshot (状态)
      if (resource === 'order_sync') {
        if (data) {
          const { event, snapshot } = data as {
            event: import('@/core/domain/types/orderEvent').OrderEvent;
            snapshot: import('@/core/domain/types/orderEvent').OrderSnapshot;
          };
          console.log(`[SyncListener] Order sync: ${event.event_type}, order=${snapshot.order_id}`);
          useActiveOrdersStore.getState()._applyOrderSync(event, snapshot);
        }
        return;
      }

      // 调用 resources store 的 applySync（传入完整 payload）
      const store = storeRegistry[resource];
      if (store) {
        console.log(`[SyncListener] Found store for ${resource}, calling applySync`);
        store.getState().applySync({ id, version, action, data });
      } else {
        console.warn(`[SyncListener] No store found for resource: ${resource}`);
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
      window.removeEventListener('order-sync-gap-detected', handleGapEvent);
    };
  }, []);
}
