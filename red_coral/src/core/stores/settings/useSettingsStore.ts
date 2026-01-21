import { persist } from 'zustand/middleware';
import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import type { EmbeddedSpec } from '@/core/domain/types';

/**
 * Settings UI Store - 纯 UI 状态管理
 *
 * 数据获取: 使用 @/core/stores/resources (useZoneStore, useTableStore, etc.)
 * 本 Store 仅管理: 导航、Modal、表单、筛选/分页 UI 状态
 */

type SettingsCategory = 'LANG' | 'PRINTER' | 'TABLES' | 'PRODUCTS' | 'CATEGORIES' | 'ATTRIBUTES' | 'DATA_TRANSFER' | 'STORE' | 'SYSTEM' | 'USERS';
type ModalAction = 'CREATE' | 'EDIT' | 'DELETE';
type ModalEntity = 'TABLE' | 'ZONE' | 'PRODUCT' | 'CATEGORY';

interface StoreInfo {
  name: string;
  address: string;
  nif: string;
  logoUrl?: string;
  phone?: string;
  email?: string;
  website?: string;
}

interface ModalState {
  open: boolean;
  action: ModalAction;
  entity: ModalEntity;
  data: any;
}

interface FormData {
  tempSpecifications: any;
  id?: string;
  name: string;
  receiptName?: string;
  capacity: number;
  zoneId: string;
  price: number;
  image: string;
  categoryId?: string | number;
  externalId?: number;
  taxRate: number;
  surchargeType: 'percentage' | 'fixed' | 'none';
  surchargeAmount: number;
  sortOrder?: number;
  selectedAttributeIds?: string[];
  attributeDefaultOptions?: Record<string, string[]>;
  kitchenPrinterId?: number | null;
  kitchenPrintName?: string;
  isKitchenPrintEnabled?: number | null;
  isLabelPrintEnabled?: number | null;
  hasMultiSpec?: boolean;
  // Loaded from getProductFull API (embedded specs)
  loadedSpecs?: EmbeddedSpec[];
  selectedTagIds?: string[];
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
  openModal: (entity: ModalEntity, action: ModalAction, data?: any) => void;
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
  tempSpecifications: [],
  id: undefined,
  name: '',
  receiptName: '',
  capacity: 4,
  zoneId: '',
  price: 0,
  image: '',
  categoryId: undefined,
  externalId: undefined,
  taxRate: 10,
  surchargeType: 'none',
  surchargeAmount: 0,
  sortOrder: undefined,
  selectedAttributeIds: [],
  attributeDefaultOptions: {},
  kitchenPrinterId: undefined,
  kitchenPrintName: '',
  isKitchenPrintEnabled: -1,
  isLabelPrintEnabled: -1,
  hasMultiSpec: false,
};

function normalizeKitchenPrintTri(value: unknown): number {
  if (value === undefined || value === null) return -1;
  if (value === -1 || value === '-1') return -1;
  if (value === true || value === 'true' || value === 1 || value === '1') return 1;
  if (value === false || value === 'false' || value === 0 || value === '0') return 0;
  return -1;
}

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
      openModal: (entity, action, data = null) => {
        let formData = { ...initialFormData };

        if (entity === 'TABLE') {
          formData = {
            ...formData,
            name: data?.name || '',
            capacity: data?.capacity ?? 4,
            zoneId: data?.zoneId || data?.defaultZoneId || '',
          };
        } else if (entity === 'ZONE') {
          formData = {
            ...formData,
            name: data?.name || '',
            surchargeType: data?.surchargeType || 'none',
            surchargeAmount: data?.surchargeAmount || 0,
          };
        } else if (entity === 'PRODUCT') {
          formData = {
            ...formData,
            id: data?.id,
            name: data?.name || '',
            receiptName: data?.receiptName ?? data?.receipt_name ?? '',
            price: data?.price ?? 0,
            image: data?.image || '',
            // Support both camelCase and snake_case field names
            categoryId: data?.categoryId ?? data?.category ?? data?.defaultCategoryId,
            externalId: data?.externalId ?? data?.external_id,
            taxRate: data?.taxRate ?? data?.tax_rate ?? 10,
            sortOrder: data?.sortOrder ?? data?.sort_order,
            kitchenPrinterId: data?.kitchenPrinterId ?? data?.kitchen_printer,
            kitchenPrintName: data?.kitchenPrintName ?? data?.kitchen_print_name ?? '',
            isKitchenPrintEnabled: normalizeKitchenPrintTri(data?.isKitchenPrintEnabled ?? data?.is_kitchen_print_enabled),
            isLabelPrintEnabled: normalizeKitchenPrintTri(data?.isLabelPrintEnabled ?? data?.is_label_print_enabled),
            hasMultiSpec: data?.hasMultiSpec ?? data?.has_multi_spec ?? false,
            tempSpecifications: data?.specifications || [],
          };
        } else if (entity === 'CATEGORY') {
          formData = {
            ...formData,
            name: data?.name || '',
            kitchenPrinterId: data?.kitchenPrinterId ?? data?.kitchen_printer_id,
            isKitchenPrintEnabled: data?.isKitchenPrintEnabled ?? data?.is_kitchen_print_enabled ?? true,
            isLabelPrintEnabled: data?.isLabelPrintEnabled ?? data?.is_label_print_enabled ?? true,
            selectedAttributeIds: data?.selectedAttributeIds || [],
            attributeDefaultOptions: data?.attributeDefaultOptions || {},
          };
        }

        set({
          modal: { open: true, action, entity, data },
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
  }
  return false;
}
