import { persist } from 'zustand/middleware';
import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import type { EmbeddedSpec, Category, Product, Tag, DiningTable, Zone } from '@/core/domain/types';

/**
 * Settings UI Store - 纯 UI 状态管理
 *
 * 数据获取: 使用 @/core/stores/resources (useZoneStore, useTableStore, etc.)
 * 本 Store 仅管理: 导航、Modal、表单、筛选/分页 UI 状态
 */

type SettingsCategory = 'LANG' | 'PRINTER' | 'TABLES' | 'PRODUCTS' | 'CATEGORIES' | 'TAGS' | 'ATTRIBUTES' | 'DATA_TRANSFER' | 'STORE' | 'SYSTEM' | 'USERS';
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
  surchargeType?: 'percentage' | 'fixed' | 'none';
  surchargeAmount?: number;
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

interface StoreInfo {
  name: string;
  address: string;
  nif: string;
  logoUrl?: string;
  phone?: string;
  email?: string;
  website?: string;
}

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
  is_label_print_enabled?: boolean;
  tags?: string[];         // Tag IDs
  specs?: EmbeddedSpec[];  // 嵌入式规格
  has_multi_spec?: boolean; // UI only: 是否多规格

  // === Category ===
  is_virtual?: boolean;
  tag_ids?: string[];      // Virtual category tag filter
  match_mode?: 'any' | 'all';
  selected_attribute_ids?: string[];  // UI only: 已选属性
  attribute_default_options?: Record<string, string[]>;  // UI only

  // === Tag ===
  color?: string;
  display_order?: number;

  // === Zone (UI only, no API fields) ===
  description?: string;
  surcharge_type?: 'percentage' | 'fixed' | 'none';
  surcharge_amount?: number;
}

interface SettingsStore {
  // Navigation
  activeCategory: SettingsCategory;
  setActiveCategory: (category: SettingsCategory) => void;

  // Store Info (persisted)
  storeInfo: StoreInfo;
  setStoreInfo: (info: StoreInfo) => void;

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
  is_label_print_enabled: false,
  tags: [],
  specs: [],
  has_multi_spec: false,
  // Category
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
  surcharge_type: 'none',
  surcharge_amount: 0,
};

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set, get) => ({
      // Navigation
      activeCategory: 'LANG',
      setActiveCategory: (category) => set({ activeCategory: category }),

      // Store Info
      storeInfo: { name: 'Red Coral POS', address: '', nif: '' },
      setStoreInfo: (info) => set({ storeInfo: info }),

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
            capacity: tableData?.capacity ?? 4,
            // DiningTable API 返回 'zone' 字段，创建时使用 'defaultZoneId'
            zoneId: tableData?.zone || tableData?.defaultZoneId || '',
          };
        } else if (entity === 'ZONE') {
          const zoneData = data as ZoneEditData | null;
          formData = {
            ...formData,
            name: zoneData?.name || '',
            surchargeType: zoneData?.surchargeType || 'none',
            surchargeAmount: zoneData?.surchargeAmount || 0,
          };
        } else if (entity === 'PRODUCT') {
          const productData = data as ProductEditData | null;
          // Product 数据使用 snake_case 字段名，需要转换
          const printDests = productData?.print_destinations || [];
          const hasKitchenPrint = printDests.length > 0;
          // 从 specs 中获取默认规格的价格
          const defaultSpec = productData?.specs?.find((s) => s.is_default) ?? productData?.specs?.[0];
          formData = {
            ...formData,
            id: productData?.id ?? undefined,
            name: productData?.name || '',
            receiptName: productData?.receipt_name ?? '',
            price: defaultSpec?.price ?? 0,
            image: productData?.image || '',
            categoryId: productData?.category ?? productData?.defaultCategoryId,
            externalId: defaultSpec?.external_id ?? undefined,
            taxRate: productData?.tax_rate ?? 10,
            sortOrder: productData?.sort_order,
            kitchenPrinterId: hasKitchenPrint ? printDests[0] : undefined,
            kitchenPrintName: productData?.kitchen_print_name ?? '',
            isKitchenPrintEnabled: hasKitchenPrint ? 1 : (productData?.print_destinations ? 0 : -1),
            isLabelPrintEnabled: normalizeKitchenPrintTri(productData?.is_label_print_enabled),
            hasMultiSpec: (productData?.specs?.length ?? 0) > 1,
            tempSpecifications: productData?.specs || [],
          };
        } else if (entity === 'CATEGORY') {
          const categoryData = data as CategoryEditData | null;
          // Category 数据中使用 print_destinations 数组，需要转换为表单字段
          const printDests = categoryData?.print_destinations || [];
          const hasKitchenPrint = printDests.length > 0;
          formData = {
            ...formData,
            name: categoryData?.name || '',
            // 从 print_destinations[0] 获取厨房打印机 ID
            kitchenPrinterId: hasKitchenPrint ? printDests[0] : undefined,
            // 根据 print_destinations 是否有值判断是否启用厨房打印 (1=启用, 0=禁用)
            isKitchenPrintEnabled: hasKitchenPrint ? 1 : 0,
            isLabelPrintEnabled: categoryData?.is_label_print_enabled ? 1 : 0,
            selectedAttributeIds: categoryData?.selectedAttributeIds || [],
            attributeDefaultOptions: categoryData?.attributeDefaultOptions || {},
            isVirtual: categoryData?.is_virtual ?? false,
            tagIds: categoryData?.tag_ids ?? [],
            matchMode: categoryData?.match_mode ?? 'any',
          };
        } else if (entity === 'TAG') {
          const tagData = data as TagEditData | null;
          formData = {
            ...formData,
            name: tagData?.name || '',
            color: tagData?.color || '#3B82F6',
            // Tag API 返回 display_order (snake_case)
            displayOrder: tagData?.display_order ?? 0,
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
      partialize: (state) => ({
        storeInfo: state.storeInfo,
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

export const useStoreInfo = () =>
  useSettingsStore(
    useShallow((state) => ({
      info: state.storeInfo,
      setInfo: state.setStoreInfo,
    }))
  );

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
    if (!formData.zoneId?.trim()) errors.zoneId = 'settings.errors.table.zoneRequired';
    if ((formData.capacity ?? 0) < 1) errors.capacity = 'settings.errors.table.capacityMin';
  } else if (entity === 'ZONE') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.zone.nameRequired';
  } else if (entity === 'PRODUCT') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.product.nameRequired';
    if (!formData.categoryId) errors.categoryId = 'settings.errors.product.categoryRequired';
    if ((formData.price ?? 0) <= 0) errors.price = 'settings.errors.product.pricePositive';
    if (formData.externalId === undefined || formData.externalId === null) {
      errors.externalId = 'settings.errors.product.externalIdRequired';
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
    const keys: (keyof FormData)[] = ['name', 'zoneId', 'capacity'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'ZONE') {
    const keys: (keyof FormData)[] = ['name', 'surchargeType', 'surchargeAmount'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'PRODUCT') {
    const keys: (keyof FormData)[] = [
      'name', 'categoryId', 'price', 'externalId', 'receiptName', 'image', 'taxRate',
      'sortOrder', 'kitchenPrinterId', 'kitchenPrintName', 'isKitchenPrintEnabled',
      'isLabelPrintEnabled', 'hasMultiSpec', 'selectedAttributeIds', 'attributeDefaultOptions',
    ];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'CATEGORY') {
    const keys: (keyof FormData)[] = [
      'name', 'kitchenPrinterId', 'isKitchenPrintEnabled', 'isLabelPrintEnabled',
      'selectedAttributeIds', 'attributeDefaultOptions',
    ];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'TAG') {
    const keys: (keyof FormData)[] = ['name', 'color', 'displayOrder'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  }
  return false;
}
