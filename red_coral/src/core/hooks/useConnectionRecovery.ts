import { useEffect, useRef } from 'react';
import { useBridgeConnectionStatus } from '@/core/stores/bridge';
import { useProductStore } from '@/core/stores/product/useProductStore';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';

/**
 * 连接恢复时自动刷新所有已加载的数据
 *
 * 当检测到从断开到连接的状态转换时，刷新所有已加载的 stores
 * 这是因为断连期间可能错过了很多 Sync 信号，或者服务可能重启过
 */
export function useConnectionRecovery() {
  const connectionStatus = useBridgeConnectionStatus();
  const prevConnected = useRef(connectionStatus.connected);

  useEffect(() => {
    // 检测从断开到连接的转换
    if (!prevConnected.current && connectionStatus.connected) {
      console.log('[Sync] Connection recovered, refreshing all loaded stores');
      refreshAllLoadedStores();
    }
    prevConnected.current = connectionStatus.connected;
  }, [connectionStatus.connected]);
}

function refreshAllLoadedStores() {
  // Product store
  const productStore = useProductStore.getState();
  if (productStore.isLoaded) {
    console.log('[Sync] Refreshing product store');
    productStore.loadProducts();
  }

  // Settings store (zones and tables)
  const settingsStore = useSettingsStore.getState();
  if (settingsStore.isLoaded) {
    console.log('[Sync] Refreshing settings store');
    settingsStore.refreshData();
  }
}
