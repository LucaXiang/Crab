/**
 * Store Registry - 资源 Store 注册表
 *
 * 用于 useSyncListener 根据资源类型分发同步信号。
 * key 必须与后端 broadcast_sync 的 resource 参数一致。
 */
import { useProductStore } from './useProductStore';
import { useCategoryStore } from './useCategoryStore';
import { useTagStore } from './useTagStore';
import { useAttributeStore } from './useAttributeStore';
import { useZoneStore } from './useZoneStore';
import { useTableStore } from './useTableStore';
import { useEmployeeStore } from './useEmployeeStore';
import { useRoleStore } from './useRoleStore';
import { usePriceRuleStore } from './usePriceRuleStore';
import { useKitchenPrinterStore } from './useKitchenPrinterStore';

// Store interface for registry
interface RegistryStore {
  getState: () => {
    isLoaded: boolean;
    fetchAll: () => Promise<void>;
    applySync: (id: string) => void;
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
 * - 其他: price_rule, kitchen_printer
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
  kitchen_printer: useKitchenPrinterStore,
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
