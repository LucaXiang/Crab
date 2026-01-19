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
import { useSpecStore } from './useSpecStore';
import { useZoneStore } from './useZoneStore';
import { useTableStore } from './useTableStore';
import { useEmployeeStore } from './useEmployeeStore';
import { useRoleStore } from './useRoleStore';
import { usePriceRuleStore } from './usePriceRuleStore';
import { useKitchenPrinterStore } from './useKitchenPrinterStore';
import { useOrderStore } from './useOrderStore';

// Store interface for registry
interface RegistryStore {
  getState: () => {
    isLoaded: boolean;
    fetchAll: () => Promise<void>;
    applySync: () => void;
    clear: () => void;
  };
}

/**
 * 资源 Store 注册表
 *
 * 13 种资源类型:
 * - 菜单相关: product, category, tag, attribute, spec
 * - 位置相关: zone, table
 * - 人员相关: employee, role
 * - 其他: price_rule, kitchen_printer, order
 */
export const storeRegistry: Record<string, RegistryStore> = {
  product: useProductStore,
  category: useCategoryStore,
  tag: useTagStore,
  attribute: useAttributeStore,
  spec: useSpecStore,
  zone: useZoneStore,
  table: useTableStore,
  employee: useEmployeeStore,
  role: useRoleStore,
  price_rule: usePriceRuleStore,
  kitchen_printer: useKitchenPrinterStore,
  order: useOrderStore,
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
  console.log(`[Sync] Refreshing ${loadedStores.length} loaded stores`);

  await Promise.all(
    loadedStores.map(([name, store]) => {
      console.log(`[Sync] Refreshing ${name} store`);
      return store.getState().fetchAll();
    })
  );
}

/**
 * 清空所有 Store
 */
export function clearAllStores(): void {
  Object.entries(storeRegistry).forEach(([name, store]) => {
    console.log(`[Store] Clearing ${name}`);
    store.getState().clear();
  });
}
