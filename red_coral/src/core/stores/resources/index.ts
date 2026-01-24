/**
 * Resource Stores - 服务器权威模型
 *
 * 所有资源 Store 统一导出。
 * 组件应直接订阅这些 Store，禁止 useState + useEffect 手动 fetch。
 */

// Factory
export {
  createResourceStore,
  createCrudResourceStore,
  type ResourceStore,
  type CrudResourceStore,
  type CrudOperations,
} from '../factory/createResourceStore';

// Registry
export {
  storeRegistry,
  getLoadedStores,
  refreshAllLoadedStores,
  clearAllStores,
} from './registry';

// Product - re-exported from feature module
export {
  useProductStore,
  useProducts,
  useProductsLoading,
  useProductById,
  useProductActions,
} from '@/features/product';

// Category - re-exported from feature module
export {
  useCategoryStore,
  useCategories,
  useCategoriesLoading,
  useCategoryById,
  useCategoryActions,
  // Virtual/Regular category selectors
  useVirtualCategories,
  useRegularCategories,
  useCategoryByName,
  getVirtualCategories,
  getRegularCategories,
  getCategoryByName,
} from '@/features/category';

// Tag - re-exported from feature module
export {
  useTagStore,
  useTags,
  useTagsLoading,
  useTagById,
} from '@/features/tag';

// Attribute - re-exported from feature module
export {
  useAttributeStore,
  useAttributes,
  useAttributesLoading,
  useAttributeById,
  useAttributeActions,
  useOptionActions,
  attributeHelpers,
  useAttributeHelpers,
} from '@/features/attribute';

// Zone - re-exported from feature module
export {
  useZoneStore,
  useZones,
  useZonesLoading,
  useZoneById,
} from '@/features/zone';

// Table - re-exported from feature module
export {
  useTableStore,
  useTables,
  useTablesLoading,
  useTableById,
  useTablesByZone,
} from '@/features/table';

// Employee - re-exported from feature module
export {
  useEmployeeStore,
  useEmployees,
  useEmployeesLoading,
  useEmployeeById,
} from '@/features/user';

// Role - re-exported from feature module
export {
  useRoleStore,
  useRoles,
  useRolesLoading,
  useRoleById,
} from '@/features/role';

// PriceRule - re-exported from feature module
export {
  usePriceRuleStore,
  usePriceRules,
  usePriceRulesLoading,
  usePriceRuleById,
  useActivePriceRules,
} from '@/features/price-rule';

// PrintDestination
export {
  usePrintDestinationStore,
  usePrintDestinations,
  usePrintDestinationsLoading,
  usePrintDestinationById,
  usePrintDestinationActions,
} from './usePrintDestinationStore';

