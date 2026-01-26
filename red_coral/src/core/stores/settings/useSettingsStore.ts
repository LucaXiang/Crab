import { persist } from 'zustand/middleware';
import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import type { EmbeddedSpec, Category, Product, Tag, DiningTable, Zone, PrintState } from '@/core/domain/types';

/**
 * Settings UI Store - 纯 UI 状态管理
 *
 * 数据获取: 使用 @/core/stores/resources (useZoneStore, useTableStore, etc.)
 * 本 Store 仅管理: 导航、Modal、表单、筛选/分页 UI 状态
 */

type SettingsCategory = 'LANG' | 'PRINTER' | 'TABLES' | 'PRODUCTS' | 'CATEGORIES' | 'TAGS' | 'ATTRIBUTES' | 'PRICE_RULES' | 'DATA_TRANSFER' | 'STORE' | 'SYSTEM' | 'USERS';
type ModalAction = 'CREATE' | 'EDIT' | 'DELETE';
type ModalEntity = 'TABLE' | 'ZONE' | 'PRODUCT' | 'CATEGORY' | 'TAG';

// ============ Entity Data Types (API snake_case) ============
// 这些类型映射 API 返回的数据结构，使用 snake_case 字段名
// TypeScript 会在编译时检测字段访问错误

/** TABLE 编辑数据 - API 返回 + 创建时默认值 */
interface TableEditData extends Partial<DiningTable> {
  /** 创建时使用的默认 Zone ID */
  defaultZoneId?: string;
}

/** ZONE 编辑数据 */
interface ZoneEditData extends Partial<Zone> {
  // Zone now only uses standard fields from Zone type
}

/** PRODUCT 编辑数据 - API 返回 + 创建时默认值 */
interface ProductEditData extends Partial<Product> {
  /** 创建时使用的默认 Category ID */
  defaultCategoryId?: string;
}

/** CATEGORY 编辑数据 - API 返回 + 属性关联 */
interface CategoryEditData extends Partial<Category> {
  /** 已绑定的属性 ID 列表 */
  selectedAttributeIds?: string[];
  /** 属性默认选项映射 */
  attributeDefaultOptions?: Record<string, string[]>;
}

/** TAG 编辑数据 */
interface TagEditData extends Partial<Tag> {}

/** Entity 到数据类型的映射 */
interface EntityDataMap {
  TABLE: TableEditData;
  ZONE: ZoneEditData;
  PRODUCT: ProductEditData;
  CATEGORY: CategoryEditData;
  TAG: TagEditData;
}

// StoreInfo moved to useStoreInfoStore - fetched from server API

interface ModalState<E extends ModalEntity = ModalEntity> {
  open: boolean;
  action: ModalAction;
  entity: E;
  data: EntityDataMap[E] | null;
}

/**
 * FormData - 表单状态，字段名与后端 API 对齐 (snake_case)
 *
 * 设计原则：
 * - 字段名直接使用 API 字段名，无需转换
 * - UI-only 字段也使用 snake_case 保持一致
 */
interface FormData {
  // === Common ===
  id?: string;
  name: string;

  // === DiningTable ===
  zone?: string;           // Zone ID
  capacity?: number;

  // === Product ===
  category?: string;       // Category ID
  image?: string;
  sort_order?: number;
  tax_rate?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  print_destinations?: string[];  // PrintDestination IDs
  is_label_print_enabled?: PrintState;  // Product: -1=继承, 0=禁用, 1=启用
  tags?: string[];         // Tag IDs
  specs?: EmbeddedSpec[];  // 嵌入式规格
  has_multi_spec?: boolean; // UI only: 是否多规格
  price?: number;          // UI only: derived from specs[root].price
  externalId?: number;     // UI only: derived from specs[root].external_id

  // === Category & Product shared ===
  is_kitchen_print_enabled?: PrintState;  // Product: -1=继承, 0=禁用, 1=启用; Category: 0=禁用, 1=启用
  label_print_destinations?: string[];  // Label PrintDestination IDs
  is_virtual?: boolean;
  is_display?: boolean;     // Virtual category display in menu
  tag_ids?: string[];      // Virtual category tag filter
  match_mode?: 'any' | 'all';
  selected_attribute_ids?: string[];  // UI only: 已选属性
  attribute_default_options?: Record<string, string | string[]>;  // UI only

  // === Tag ===
  color?: string;
  display_order?: number;
  is_active?: boolean;

  // === Zone ===
  description?: string;
}

interface SettingsStore {
  // Navigation
  activeCategory: SettingsCategory;
  setActiveCategory: (category: SettingsCategory) => void;

  // Filter & Pagination UI State
  selectedZoneFilter: string | 'all';
  tablesPage: number;
  tablesTotal: number;
  productCategoryFilter: string | 'all';
  productsPage: number;
  productsTotal: number;
  setSelectedZoneFilter: (zoneId: string | 'all') => void;
  setTablesPagination: (page: number, total: number) => void;
  setProductCategoryFilter: (category: string | 'all') => void;
  setProductsPagination: (page: number, total: number) => void;

  // Modal State
  modal: ModalState;
  openModal: <E extends ModalEntity>(entity: E, action: ModalAction, data?: EntityDataMap[E] | null) => void;
  closeModal: () => void;

  // Data Refresh Signal
  dataVersion: number;
  refreshData: () => void;

  // System Settings (persisted)
  performanceMode: boolean;
  setPerformanceMode: (enabled: boolean) => void;
  lastSelectedCategory: string;
  setLastSelectedCategory: (category: string) => void;

  // Form State
  formData: FormData;
  formInitialData: FormData;
  isFormDirty: boolean;
  formErrors: Record<string, string | undefined>;
  setFormField: <K extends keyof FormData>(field: K, value: FormData[K]) => void;
  setFormData: (data: Partial<FormData>) => void;
  resetFormData: () => void;
  initFormData: (data: Partial<FormData>) => void;
  setAsyncFormData: (data: Partial<FormData>) => void;
}

const initialFormData: FormData = {
  id: undefined,
  name: '',
  // DiningTable
  zone: '',
  capacity: 4,
  // Product
  category: undefined,
  image: '',
  sort_order: undefined,
  tax_rate: 10,
  receipt_name: '',
  kitchen_print_name: '',
  print_destinations: [],
  is_label_print_enabled: -1,  // 默认继承分类
  tags: [],
  specs: [],
  has_multi_spec: false,
  // Category
  is_kitchen_print_enabled: 1,  // 默认启用
  is_virtual: false,
  tag_ids: [],
  match_mode: 'any',
  selected_attribute_ids: [],
  attribute_default_options: {},
  // Tag
  color: '#3B82F6',
  display_order: 0,
  // Zone
  description: '',
};

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set, get) => ({
      // Navigation
      activeCategory: 'LANG',
      setActiveCategory: (category) => set({ activeCategory: category }),

      // Filter & Pagination UI State
      selectedZoneFilter: 'all',
      tablesPage: 1,
      tablesTotal: 0,
      productCategoryFilter: 'all',
      productsPage: 1,
      productsTotal: 0,
      setSelectedZoneFilter: (zoneId) => set({ selectedZoneFilter: zoneId, tablesPage: 1 }),
      setTablesPagination: (page, total) => set({ tablesPage: page, tablesTotal: total }),
      setProductCategoryFilter: (category) => set({ productCategoryFilter: category, productsPage: 1 }),
      setProductsPagination: (page, total) => set({ productsPage: page, productsTotal: total }),

      // Modal State
      modal: { open: false, action: 'CREATE', entity: 'TABLE', data: null },
      openModal: <E extends ModalEntity>(entity: E, action: ModalAction, data: EntityDataMap[E] | null = null) => {
        let formData = { ...initialFormData };

        if (entity === 'TABLE') {
          const tableData = data as TableEditData | null;
          formData = {
            ...formData,
            name: tableData?.name || '',
            zone: tableData?.zone || tableData?.defaultZoneId || '',
            capacity: tableData?.capacity ?? 4,
            is_active: tableData?.is_active ?? true,  // Default to active for new tables
          };
        } else if (entity === 'ZONE') {
          const zoneData = data as ZoneEditData | null;
          formData = {
            ...formData,
            name: zoneData?.name || '',
            description: zoneData?.description || '',
            is_active: zoneData?.is_active ?? true,  // Default to active for new zones
          };
        } else if (entity === 'PRODUCT') {
          const productData = data as ProductEditData | null;
          formData = {
            ...formData,
            id: productData?.id ?? undefined,
            name: productData?.name || '',
            category: productData?.category ?? productData?.defaultCategoryId,
            image: productData?.image || '',
            sort_order: productData?.sort_order,
            tax_rate: productData?.tax_rate ?? 10,
            receipt_name: productData?.receipt_name ?? '',
            kitchen_print_name: productData?.kitchen_print_name ?? '',
            print_destinations: productData?.kitchen_print_destinations || [],  // Form uses print_destinations for kitchen
            label_print_destinations: productData?.label_print_destinations || [],
            is_kitchen_print_enabled: productData?.is_kitchen_print_enabled ?? -1,  // 默认继承分类
            is_label_print_enabled: productData?.is_label_print_enabled ?? -1,  // 默认继承分类
            is_active: productData?.is_active ?? true,  // Default to active for new products
            tags: productData?.tags || [],
            specs: productData?.specs || [],
            has_multi_spec: (productData?.specs?.length ?? 0) > 1,
          };
        } else if (entity === 'CATEGORY') {
          const categoryData = data as CategoryEditData | null;
          formData = {
            ...formData,
            name: categoryData?.name || '',
            sort_order: categoryData?.sort_order,
            print_destinations: categoryData?.kitchen_print_destinations || [],  // Form uses print_destinations for kitchen
            label_print_destinations: categoryData?.label_print_destinations || [],
            is_kitchen_print_enabled: categoryData?.is_kitchen_print_enabled ? 1 : 0,  // Category: bool → 0/1
            is_label_print_enabled: categoryData?.is_label_print_enabled ? 1 : 0,  // Category: bool → 0/1
            is_active: categoryData?.is_active ?? true,  // Default to active for new categories
            is_virtual: categoryData?.is_virtual ?? false,
            tag_ids: categoryData?.tag_ids ?? [],
            match_mode: categoryData?.match_mode ?? 'any',
            selected_attribute_ids: categoryData?.selectedAttributeIds || [],
            attribute_default_options: categoryData?.attributeDefaultOptions || {},
          };
        } else if (entity === 'TAG') {
          const tagData = data as TagEditData | null;
          formData = {
            ...formData,
            name: tagData?.name || '',
            color: tagData?.color || '#3B82F6',
            display_order: tagData?.display_order ?? 0,
            is_active: tagData?.is_active ?? true,  // Default to active for new tags
          };
        }

        set({
          modal: { open: true, action, entity, data: data as EntityDataMap[ModalEntity] | null },
          formData,
          formInitialData: { ...formData },
          isFormDirty: false,
          formErrors: {},
        });
      },
      closeModal: () => set({ modal: { open: false, action: 'CREATE', entity: 'TABLE', data: null } }),

      // Data Refresh Signal
      dataVersion: 0,
      refreshData: () => set((state) => ({ dataVersion: state.dataVersion + 1 })),

      // System Settings
      performanceMode: false,
      setPerformanceMode: (enabled) => set({ performanceMode: enabled }),
      lastSelectedCategory: '',
      setLastSelectedCategory: (category) => set({ lastSelectedCategory: category }),

      // Form State
      formData: { ...initialFormData },
      formInitialData: { ...initialFormData },
      isFormDirty: false,
      formErrors: {},
      setFormField: (field, value) => {
        const state = get();
        const nextFormData = { ...state.formData, [field]: value };
        const errors = validateSettingsForm(state.modal.entity, nextFormData);
        const isDirty = computeIsDirty(state.modal.entity, nextFormData, state.formInitialData);
        set({ formData: nextFormData, isFormDirty: isDirty, formErrors: errors });
      },
      setFormData: (data) => {
        const state = get();
        const nextFormData = { ...state.formData, ...data };
        const errors = validateSettingsForm(state.modal.entity, nextFormData);
        const isDirty = computeIsDirty(state.modal.entity, nextFormData, state.formInitialData);
        set({ formData: nextFormData, isFormDirty: isDirty, formErrors: errors });
      },
      resetFormData: () => {
        const { formInitialData, modal } = get();
        set({
          formData: { ...formInitialData },
          isFormDirty: false,
          formErrors: validateSettingsForm(modal.entity, formInitialData),
        });
      },
      initFormData: (data) => {
        const { modal } = get();
        const nextFormData = { ...initialFormData, ...data };
        set({
          formData: nextFormData,
          formInitialData: { ...nextFormData },
          isFormDirty: false,
          formErrors: validateSettingsForm(modal.entity, nextFormData),
        });
      },
      setAsyncFormData: (data) => {
        const { formData, formInitialData, modal } = get();
        const nextFormData = { ...formData, ...data };
        const nextInitialData = { ...formInitialData, ...data };
        const errors = validateSettingsForm(modal.entity, nextFormData);
        const isDirty = computeIsDirty(modal.entity, nextFormData, nextInitialData);
        set({
          formData: nextFormData,
          formInitialData: nextInitialData,
          isFormDirty: isDirty,
          formErrors: errors,
        });
      },
    }),
    {
      name: 'settings-storage',
      // TODO: storeInfo 应该从服务端获取，已从持久化中移除
      partialize: (state) => ({
        performanceMode: state.performanceMode,
        lastSelectedCategory: state.lastSelectedCategory,
      }),
    }
  )
);

// ============ Selectors ============

export const useSettingsCategory = () =>
  useSettingsStore((state) => state.activeCategory);

export const useSettingsModal = () =>
  useSettingsStore(
    useShallow((state) => ({
      modal: state.modal,
      openModal: state.openModal,
      closeModal: state.closeModal,
    }))
  );

export const useSettingsForm = () =>
  useSettingsStore(
    useShallow((state) => ({
      formData: state.formData,
      setFormField: state.setFormField,
      resetFormData: state.resetFormData,
      initFormData: state.initFormData,
    }))
  );

export const useSettingsFormMeta = () =>
  useSettingsStore(
    useShallow((state) => ({
      formData: state.formData,
      setFormField: state.setFormField,
      setFormData: state.setFormData,
      resetFormData: state.resetFormData,
      initFormData: state.initFormData,
      setAsyncFormData: state.setAsyncFormData,
      isFormDirty: state.isFormDirty,
      formErrors: state.formErrors,
    }))
  );

// useStoreInfo moved to useStoreInfoStore.ts

export const useDataVersion = () => useSettingsStore((state) => state.dataVersion);

export const useSettingsFilters = () =>
  useSettingsStore(
    useShallow((state) => ({
      selectedZoneFilter: state.selectedZoneFilter,
      tablesPage: state.tablesPage,
      tablesTotal: state.tablesTotal,
      productCategoryFilter: state.productCategoryFilter,
      productsPage: state.productsPage,
      productsTotal: state.productsTotal,
      setSelectedZoneFilter: state.setSelectedZoneFilter,
      setTablesPagination: state.setTablesPagination,
      setProductCategoryFilter: state.setProductCategoryFilter,
      setProductsPagination: state.setProductsPagination,
    }))
  );

// ============ Validation Helpers ============

function validateSettingsForm(entity: ModalEntity, formData: FormData): Record<string, string | undefined> {
  const errors: Record<string, string | undefined> = {};
  if (entity === 'TABLE') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.table.nameRequired';
    if (!formData.zone?.trim()) errors.zone = 'settings.errors.table.zoneRequired';
    if ((formData.capacity ?? 0) < 1) errors.capacity = 'settings.errors.table.capacityMin';
  } else if (entity === 'ZONE') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.zone.nameRequired';
  } else if (entity === 'PRODUCT') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.product.nameRequired';
    if (!formData.category) errors.category = 'settings.errors.product.categoryRequired';
    // Check external_id in default spec
    const defaultSpec = formData.specs?.find(s => s.is_default) ?? formData.specs?.[0];
    if (defaultSpec?.external_id === undefined || defaultSpec?.external_id === null) {
      errors.externalId = 'settings.external_id_required';
    }
  } else if (entity === 'CATEGORY') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.category.nameRequired';
  } else if (entity === 'TAG') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.tag.nameRequired';
  }
  return errors;
}

function computeIsDirty(entity: ModalEntity, next: FormData, initial: FormData): boolean {
  const pick = (o: FormData, keys: (keyof FormData)[]) => keys.map((k) => o[k]);

  if (entity === 'TABLE') {
    const keys: (keyof FormData)[] = ['name', 'zone', 'capacity', 'is_active'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'ZONE') {
    const keys: (keyof FormData)[] = ['name', 'description', 'is_active'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'PRODUCT') {
    const keys: (keyof FormData)[] = [
      'name', 'category', 'image', 'tax_rate', 'receipt_name',
      'sort_order', 'print_destinations', 'label_print_destinations',
      'kitchen_print_name', 'is_kitchen_print_enabled', 'is_label_print_enabled',
      'is_active', 'has_multi_spec', 'tags', 'specs',
      'selected_attribute_ids', 'attribute_default_options',
    ];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'CATEGORY') {
    const keys: (keyof FormData)[] = [
      'name', 'sort_order', 'print_destinations',
      'is_kitchen_print_enabled', 'is_label_print_enabled',
      'is_active', 'is_virtual', 'tag_ids', 'match_mode',
      'selected_attribute_ids', 'attribute_default_options',
    ];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'TAG') {
    const keys: (keyof FormData)[] = ['name', 'color', 'display_order', 'is_active'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  }
  return false;
}
