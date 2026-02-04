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
import { t } from '@/infrastructure/i18n';

// 核心资源：启动时预加载（带 i18n key 以便报错）
const CORE_STORES: { key: string; fetch: () => Promise<unknown> }[] = [
  { key: 'common.resource.zone', fetch: () => useZoneStore.getState().fetchAll() },
  { key: 'common.resource.table', fetch: () => useTableStore.getState().fetchAll() },
  { key: 'common.resource.category', fetch: () => useCategoryStore.getState().fetchAll() },
  { key: 'common.resource.product', fetch: () => useProductStore.getState().fetchAll() },
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
        .map((r, i) => (r.status === 'rejected' ? t(CORE_STORES[i].key) : null))
        .filter((name): name is string => name !== null);

      if (failed.length > 0) {
        toast.error(t('app.init.load_failed', { resources: failed.join(', ') }));
      }

      setReady(true);
    };

    preload();
  }, []);

  return ready;
}
