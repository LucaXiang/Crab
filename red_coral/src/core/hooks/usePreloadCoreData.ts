/**
 * Preload Core Data Hook - 预加载核心数据
 *
 * 启动时预加载 POS 必需的核心资源：
 * - zone: 区域数据
 * - table: 桌台数据
 * - category: 分类数据
 * - product: 产品数据
 *
 * 其他资源按需加载。
 */

import { useState, useEffect } from 'react';
import { useZoneStore } from '@/core/stores/resources/useZoneStore';
import { useTableStore } from '@/core/stores/resources/useTableStore';
import { useCategoryStore } from '@/core/stores/resources/useCategoryStore';
import { useProductStore } from '@/core/stores/resources/useProductStore';

// 核心资源：启动时预加载
const CORE_STORES = [
  useZoneStore,
  useTableStore,
  useCategoryStore,
  useProductStore,
];

/**
 * 预加载核心数据
 *
 * @returns ready - 是否预加载完成
 */
export function usePreloadCoreData(): boolean {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const preload = async () => {

      try {
        await Promise.all(
          CORE_STORES.map((store) => store.getState().fetchAll())
        );
        setReady(true);
      } catch (error) {
        console.error('[Preload] Failed to load core data:', error);
        // Still mark as ready to allow app to render with error state
        setReady(true);
      }
    };

    preload();
  }, []);

  return ready;
}
