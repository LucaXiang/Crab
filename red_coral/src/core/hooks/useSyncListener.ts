/**
 * Sync Listener Hook - 服务器权威模型
 *
 * 监听 server-message 事件中的 Sync 信号，触发 Store 刷新。
 * 使用 resources/ 下的统一 Store 架构。
 */

import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { storeRegistry } from '@/core/stores/resources/registry';

interface SyncPayload {
  resource: string;
  action: string;
  id: string;
  data: unknown | null;
}

interface ServerMessageEvent {
  event_type: string;
  payload: SyncPayload;
  correlation_id: string | null;
}

/**
 * 监听同步信号，触发 Store 刷新
 *
 * 服务器权威：收到 Sync 信号直接全量刷新对应资源
 */
export function useSyncListener() {
  useEffect(() => {
    const unlisten = listen<ServerMessageEvent>('server-message', (event) => {
      const message = event.payload;

      // Only handle Sync type messages
      if (message.event_type !== 'Sync') return;

      const { resource, action, id } = message.payload;

      // 调用 resources store 的 applySync（传入 id 用于去重）
      const store = storeRegistry[resource];
      if (store) {
        store.getState().applySync(id);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
