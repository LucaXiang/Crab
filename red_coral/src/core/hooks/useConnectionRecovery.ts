/**
 * Connection Recovery Hook - 连接恢复自动刷新
 *
 * 监听连接状态变化，当检测到从断开到连接的状态转换时，
 * 刷新所有已加载的 Stores。
 *
 * 使用 resources/ 下的统一 Store 架构。
 */

import { useEffect, useRef } from 'react';
import { useBridgeConnectionStatus } from '@/core/stores/bridge';
import { refreshAllLoadedStores } from '@/core/stores/resources/registry';

/**
 * 连接恢复时自动刷新所有已加载的数据
 */
export function useConnectionRecovery() {
  const connectionStatus = useBridgeConnectionStatus();
  const prevConnected = useRef(connectionStatus.connected);

  useEffect(() => {
    // 检测从断开到连接的转换
    if (!prevConnected.current && connectionStatus.connected) {
      refreshAllLoadedStores();
    }
    prevConnected.current = connectionStatus.connected;
  }, [connectionStatus.connected]);
}
