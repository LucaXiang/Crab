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
import { useZoneStore } from '@/features/zone';
import { useTableStore } from '@/features/table';
import { useCategoryStore } from '@/features/category';
import { useProductStore } from '@/features/product';
import { toast } from '@/presentation/components/Toast';

// 核心资源：启动时预加载（带名称以便报错）
const CORE_STORES: { name: string; fetch: () => Promise<unknown> }[] = [
  { name: '区域', fetch: () => useZoneStore.getState().fetchAll() },
  { name: '桌台', fetch: () => useTableStore.getState().fetchAll() },
  { name: '分类', fetch: () => useCategoryStore.getState().fetchAll() },
  { name: '商品', fetch: () => useProductStore.getState().fetchAll() },
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
      const results = await Promise.allSettled(
        CORE_STORES.map((store) => store.fetch())
      );

      const failed = results
        .map((r, i) => (r.status === 'rejected' ? CORE_STORES[i].name : null))
        .filter((name): name is string => name !== null);

      if (failed.length > 0) {
        toast.error(`核心数据加载失败: ${failed.join('、')}`);
      }

      setReady(true);
    };

    preload();
  }, []);

  return ready;
}
