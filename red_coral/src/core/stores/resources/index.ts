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

// Product
export {
  useProductStore,
  useProducts,
  useProductsLoading,
  useProductById,
  useProductActions,
} from './useProductStore';

// Category
export {
  useCategoryStore,
  useCategories,
  useCategoriesLoading,
  useCategoryById,
  useCategoryActions,
} from './useCategoryStore';

// Tag
export {
  useTagStore,
  useTags,
  useTagsLoading,
  useTagById,
} from './useTagStore';

// Attribute
export {
  useAttributeStore,
  useAttributes,
  useAttributesLoading,
  useAttributeById,
  useAttributeActions,
  useOptionActions,
  attributeHelpers,
  useAttributeHelpers,
} from './useAttributeStore';

// Zone
export {
  useZoneStore,
  useZones,
  useZonesLoading,
  useZoneById,
} from './useZoneStore';

// Table
export {
  useTableStore,
  useTables,
  useTablesLoading,
  useTableById,
  useTablesByZone,
} from './useTableStore';

// Employee
export {
  useEmployeeStore,
  useEmployees,
  useEmployeesLoading,
  useEmployeeById,
} from './useEmployeeStore';

// Role
export {
  useRoleStore,
  useRoles,
  useRolesLoading,
  useRoleById,
} from './useRoleStore';

// PriceRule
export {
  usePriceRuleStore,
  usePriceRules,
  usePriceRulesLoading,
  usePriceRuleById,
  useActivePriceRules,
} from './usePriceRuleStore';

// KitchenPrinter
export {
  useKitchenPrinterStore,
  useKitchenPrinters,
  useKitchenPrintersLoading,
  useKitchenPrinterById,
  useKitchenPrinterActions,
} from './useKitchenPrinterStore';

