/**
 * Resource Stores - 服务器权威模型
 *
 * 所有资源 Store 统一导出。
 * 组件应直接订阅这些 Store，禁止 useState + useEffect 手动 fetch。
 */

// Registry
export {
  storeRegistry,
  getLoadedStores,
  refreshAllLoadedStores,
} from './registry';

// Product - re-exported from feature module
export {
  useProductStore,
  useProducts,
  useProductsLoading,
} from '@/features/product';

// Category - re-exported from feature module
export {
  useCategoryStore,
  useCategories,
} from '@/features/category';

// Tag - re-exported from feature module
export {
  useTagStore,
  useTags,
} from '@/features/tag';

// Attribute - re-exported from feature module
export {
  useAttributeStore,
  useAttributes,
  useAttributeActions,
  useOptionActions,
  attributeHelpers,
} from '@/features/attribute';

// Zone - re-exported from feature module
export {
  useZoneStore,
  useZones,
} from '@/features/zone';

// Table - re-exported from feature module
export {
  useTableStore,
  useTables,
} from '@/features/table';

// Role - re-exported from feature module
export {
  useRoles,
} from '@/features/role';

// PrintDestination
export {
  usePrintDestinationStore,
} from './usePrintDestinationStore';
