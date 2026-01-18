/**
 * Sync Listener Hook
 *
 * Listens for server-message events with Sync payloads and updates local stores.
 * Uses a threshold-based approach: small version gaps apply incremental updates,
 * large gaps trigger full data refresh.
 */

import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { useProductStore } from '@/core/stores/product/useProductStore';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';

const SYNC_THRESHOLD = 5;

interface SyncPayload {
  resource: string;
  version: number;
  action: string;
  id: string;
  data: any | null;
}

interface ServerMessageEvent {
  event_type: string;
  payload: any;
  correlation_id: string | null;
}

export function useSyncListener() {
  useEffect(() => {
    const unlisten = listen<ServerMessageEvent>('server-message', (event) => {
      const message = event.payload;

      // Only handle Sync type messages
      if (message.event_type !== 'Sync') return;

      const syncPayload = message.payload as SyncPayload;
      const { resource, version, action, id, data } = syncPayload;

      console.log(`[Sync] Received: ${resource} ${action} ${id} (v${version})`);

      // Dispatch to corresponding store based on resource type
      switch (resource) {
        case 'product': {
          const store = useProductStore.getState();
          if (!store.isLoaded) return;

          const localVersion = store.dataVersion || 0;
          const gap = version - localVersion;

          if (gap <= 0) return;

          if (gap <= SYNC_THRESHOLD) {
            store.applySync(action, id, data);
            store.setVersion(version);
          } else {
            console.log(`[Sync] Version gap ${gap} > ${SYNC_THRESHOLD}, full refresh for ${resource}`);
            store.loadProducts();
          }
          break;
        }

        case 'zone': {
          const store = useSettingsStore.getState();
          if (!store.isLoaded) return;

          const localVersion = store.dataVersion || 0;
          const gap = version - localVersion;

          if (gap <= 0) return;

          if (gap <= SYNC_THRESHOLD) {
            store.applySyncZone(action, id, data);
            store.setDataVersion(version);
          } else {
            console.log(`[Sync] Version gap ${gap} > ${SYNC_THRESHOLD}, full refresh for ${resource}`);
            // Trigger full data refresh via dataVersion increment
            // Components listening to dataVersion will refetch zones
            store.refreshData();
          }
          break;
        }

        case 'table': {
          const store = useSettingsStore.getState();
          if (!store.isLoaded) return;

          const localVersion = store.dataVersion || 0;
          const gap = version - localVersion;

          if (gap <= 0) return;

          if (gap <= SYNC_THRESHOLD) {
            store.applySyncTable(action, id, data);
            store.setDataVersion(version);
          } else {
            console.log(`[Sync] Version gap ${gap} > ${SYNC_THRESHOLD}, full refresh for ${resource}`);
            // Trigger full data refresh via dataVersion increment
            // Components listening to dataVersion will refetch tables
            store.refreshData();
          }
          break;
        }

        default:
          console.log(`[Sync] Unknown resource: ${resource}`);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
