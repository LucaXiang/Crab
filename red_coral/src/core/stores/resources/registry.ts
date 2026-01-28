/**
 * Store Registry - 资源 Store 注册表
 *
 * 用于 useSyncListener 根据资源类型分发同步信号。
 * key 必须与后端 broadcast_sync 的 resource 参数一致。
 */
import type { SyncPayload } from '../factory/createResourceStore';
import { useProductStore } from '@/features/product';
import { useCategoryStore } from '@/features/category/store';
import { useTagStore } from '@/features/tag';
import { useAttributeStore } from '@/features/attribute';
import { useZoneStore } from '@/features/zone';
import { useTableStore } from '@/features/table';
import { useEmployeeStore } from '@/features/user';
import { useRoleStore } from '@/features/role';
import { usePriceRuleStore } from '@/features/price-rule';
import { usePrintDestinationStore } from './usePrintDestinationStore';
import { useStoreInfoStore } from '../settings/useStoreInfoStore';
import { useLabelTemplateStore } from '../printer/useLabelTemplateStore';

// Store interface for registry
// Note: Uses SyncPayload<any> to be compatible with all typed stores (contravariance)
// lastVersion and checkVersion are optional for legacy stores not yet updated
interface RegistryStore {
  getState: () => {
    isLoaded: boolean;
    lastVersion?: number;
    fetchAll: (force?: boolean) => Promise<void>;
    applySync: (payload: SyncPayload<any>) => void;
    checkVersion?: (serverVersion: number) => boolean;
    clear: () => void;
  };
}

/**
 * 资源 Store 注册表
 *
 * key 必须与后端 broadcast_sync 的 resource 参数完全一致！
 *
 * 10 种资源类型:
 * - 菜单相关: product, category, tag, attribute
 * - 位置相关: zone, dining_table
 * - 人员相关: employee, role (role 无 sync，只读)
 * - 其他: price_rule, print_destination
 */
export const storeRegistry: Record<string, RegistryStore> = {
  product: useProductStore,
  category: useCategoryStore,
  tag: useTagStore,
  attribute: useAttributeStore,
  zone: useZoneStore,
  dining_table: useTableStore,          // 后端: RESOURCE = "dining_table"
  employee: useEmployeeStore,
  role: useRoleStore,                    // 后端无 sync (只读 API)
  price_rule: usePriceRuleStore,
  print_destination: usePrintDestinationStore,
  store_info: useStoreInfoStore,        // 店铺信息
  label_template: useLabelTemplateStore, // 标签模板
};

/**
 * 获取所有已加载的 Store
 */
export function getLoadedStores(): [string, RegistryStore][] {
  return Object.entries(storeRegistry).filter(
    ([, store]) => store.getState().isLoaded
  );
}

/**
 * 刷新所有已加载的 Store
 */
export async function refreshAllLoadedStores(): Promise<void> {
  const loadedStores = getLoadedStores();

  await Promise.all(
    loadedStores.map(([name, store]) => {
      return store.getState().fetchAll();
    })
  );
}

/**
 * 清空所有 Store
 */
export function clearAllStores(): void {
  Object.entries(storeRegistry).forEach(([name, store]) => {
    store.getState().clear();
  });
}
