import { persist } from 'zustand/middleware';
import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { Category } from '@/core/domain/types';

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

interface Zone {
  id: string;
  name: string;
  surchargeType?: 'percentage' | 'fixed';
  surchargeAmount?: number;
}

interface Table {
  id: string;
  name: string;
  zoneId?: string;
  zone_id?: string;
  capacity?: number;
  seats?: number;
}

interface Product {
  id: string;
  name: string;
  price: number;
  image: string;
  category: string;
  externalId: number;
  taxRate?: number;
  receiptName?: string;
  sortOrder?: number;
  kitchenPrinterId?: number | null;
  kitchenPrintName?: string;
  isKitchenPrintEnabled?: number | null;
  isLabelPrintEnabled?: number | null;
}

interface ModalState {
  open: boolean;
  action: ModalAction;
  entity: ModalEntity;
  data: any;
}

interface SettingsStore {
  // Active Category
  activeCategory: SettingsCategory;
  setActiveCategory: (category: SettingsCategory) => void;

  // Store Info
  storeInfo: StoreInfo;
  setStoreInfo: (info: StoreInfo) => void;

  // Zones Data
  zones: Zone[];
  zonesLoading: boolean;
  setZones: (zones: Zone[]) => void;
  setZonesLoading: (loading: boolean) => void;

  // Tables Data
  tables: Table[];
  tablesLoading: boolean;
  selectedZoneFilter: string | 'all';
  tablesPage: number;
  tablesTotal: number;
  setTables: (tables: Table[]) => void;
  setTablesLoading: (loading: boolean) => void;
  setSelectedZoneFilter: (zoneId: string | 'all') => void;
  setTablesPagination: (page: number, total: number) => void;

  // Categories Data
  categories: Category[];
  categoriesLoading: boolean;
  setCategories: (categories: Category[]) => void;
  setCategoriesLoading: (loading: boolean) => void;

  // Products Data
  products: Product[];
  productsLoading: boolean;
  productCategoryFilter: string | 'all';
  productsPage: number;
  productsTotal: number;
  productsVersion: number;
  setProducts: (products: Product[]) => void;
  setProductsLoading: (loading: boolean) => void;
  setProductCategoryFilter: (category: string | 'all') => void;
  setProductsPagination: (page: number, total: number) => void;
  updateProductInList: (productId: string, updates: Partial<Product>) => void;
  removeProductFromList: (productId: string) => void;

  // Modal State
  modal: ModalState;
  openModal: (entity: ModalEntity, action: ModalAction, data?: any) => void;
  closeModal: () => void;

  // Global Data Refresh
  dataVersion: number;
  isLoaded: boolean;
  refreshData: () => void;
  refreshProductsOnly: () => void;

  // Sync Support
  applySyncZone: (action: string, id: string, data: Zone | null) => void;
  applySyncTable: (action: string, id: string, data: Table | null) => void;
  setDataVersion: (version: number) => void;
  setIsLoaded: (loaded: boolean) => void;

  // System Settings
  performanceMode: boolean;
  setPerformanceMode: (enabled: boolean) => void;
  lastSelectedCategory: string;
  setLastSelectedCategory: (category: string) => void;

  // Form Fields (unified for all entities)
  formData: {
    tempSpecifications: any;
    id?: string; // Entity ID (for editing)
    name: string;
    receiptName?: string;
    capacity: number;
    zoneId: string;
    price: number;
    image: string;
    categoryId?: number; // Product/Category ID reference
    externalId?: number;
    taxRate: number;
    surchargeType: 'percentage' | 'fixed' | 'none';
    surchargeAmount: number;
    sortOrder?: number;
    selectedAttributeIds?: string[];  // For product/category attribute binding
    attributeDefaultOptions?: Record<string, string[]>; // Product/Category-level default options
    // Kitchen Printer settings (shared by Product & Category)
    kitchenPrinterId?: number | null;
    kitchenPrintName?: string;
    isKitchenPrintEnabled?: number | null;
    isLabelPrintEnabled?: number | null;
    // Multi-specification support (Product only)
    hasMultiSpec?: boolean;
  };
  formInitialData: SettingsStore['formData'];
  isFormDirty: boolean;
  formErrors: Record<string, string | undefined>;
  setFormField: <K extends keyof SettingsStore['formData']>(
    field: K,
    value: SettingsStore['formData'][K]
  ) => void;
  setFormData: (data: Partial<SettingsStore['formData']>) => void;
  resetFormData: () => void;
  initFormData: (data: Partial<SettingsStore['formData']>) => void;
  setAsyncFormData: (data: Partial<SettingsStore['formData']>) => void;
}

const initialFormData = {
  tempSpecifications: [],
  id: undefined as string | undefined,
  name: '',
  receiptName: '',
  capacity: 4,
  zoneId: '',
  price: 0,
  image: '',
  categoryId: undefined as number | undefined,
  externalId: undefined,
  taxRate: 0.10,
  surchargeType: 'none' as const,
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
      // Active Category
      activeCategory: 'LANG',
      setActiveCategory: (category) => set({ activeCategory: category }),

      // Store Info
      storeInfo: {
        name: 'Red Coral POS',
        address: '',
        nif: '',
      },
      setStoreInfo: (info) => set({ storeInfo: info }),

      // Zones Data
  zones: [],
  zonesLoading: false,
  setZones: (zones) => set({ zones }),
  setZonesLoading: (loading) => set({ zonesLoading: loading }),

  // Tables Data
  tables: [],
  tablesLoading: false,
  selectedZoneFilter: 'all',
  tablesPage: 1,
  tablesTotal: 0,
  setTables: (tables) => set({ tables }),
  setTablesLoading: (loading) => set({ tablesLoading: loading }),
  setSelectedZoneFilter: (zoneId) => set({ selectedZoneFilter: zoneId, tablesPage: 1 }),
  setTablesPagination: (page, total) => set({ tablesPage: page, tablesTotal: total }),

  // Categories Data
  categories: [],
  categoriesLoading: false,
  setCategories: (categories) => set({ categories }),
  setCategoriesLoading: (loading) => set({ categoriesLoading: loading }),

  // Products Data
  products: [],
  productsLoading: false,
  productCategoryFilter: 'all',
  productsPage: 1,
  productsTotal: 0,
  productsVersion: 0,
  setProducts: (products) => set({ products }),
  setProductsLoading: (loading) => set({ productsLoading: loading }),
  setProductCategoryFilter: (category) => set({ productCategoryFilter: category, productsPage: 1 }),
  setProductsPagination: (page, total) => set({ productsPage: page, productsTotal: total }),
  updateProductInList: (productId, updates) => set((state) => ({
    products: state.products.map((p) => (p.id === productId ? { ...p, ...updates } : p)),
  })),
  removeProductFromList: (productId) => set((state) => ({
    products: state.products.filter((p) => p.id !== productId),
  })),

  // Modal State
  modal: {
    open: false,
    action: 'CREATE',
    entity: 'TABLE',
    data: null,
  },
  openModal: (entity, action, data = null) => {
    const { zones, categories, lastSelectedCategory } = get();
    const defaultZone = zones[0]?.id || '';
    const defaultCategoryId = lastSelectedCategory
      ? categories.find(c => c.name === lastSelectedCategory)?.id
      : categories[0]?.id;

    // Initialize form data based on entity and action
    let formData = { ...initialFormData };

    if (entity === 'TABLE') {
      formData = {
        ...formData,
        name: data?.name || '',
        capacity: data?.capacity ?? 4,
        zoneId: data?.zoneId || defaultZone,
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
        receiptName: data?.receiptName || '',
        price: data?.price ?? 0,
        image: data?.image || '',
        categoryId: data?.categoryId ?? defaultCategoryId,
        externalId: data?.externalId,
        taxRate: data?.taxRate ?? 0.10,
        sortOrder: data?.sortOrder,
        kitchenPrinterId: data?.kitchenPrinterId,
        kitchenPrintName: data?.kitchenPrintName || '',
        isKitchenPrintEnabled: normalizeKitchenPrintTri(data?.isKitchenPrintEnabled),
        isLabelPrintEnabled: normalizeKitchenPrintTri(data?.isLabelPrintEnabled),
        hasMultiSpec: data?.hasMultiSpec || false, // Multi-specification support
        tempSpecifications: data?.specifications || [],
      };
      console.log('[SettingsStore] openModal PRODUCT', {
        id: data?.id,
        rawIsKitchenPrintEnabled: data?.isKitchenPrintEnabled,
        normalizedIsKitchenPrintEnabled: formData.isKitchenPrintEnabled,
        hasMultiSpec: formData.hasMultiSpec,
      });
    } else if (entity === 'CATEGORY') {
        formData = {
          ...formData,
          name: data?.name || '',
          // Support both camelCase and snake_case from API
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
  closeModal: () =>
    set({
      modal: { open: false, action: 'CREATE', entity: 'TABLE', data: null },
    }),

  // Global Data Refresh
  dataVersion: 0,
      isLoaded: false,
      refreshData: () => set((state) => ({ dataVersion: state.dataVersion + 1 })),
      refreshProductsOnly: () => set((state) => ({ productsVersion: state.productsVersion + 1 })),

      // Sync Support
      applySyncZone: (action: string, id: string, data: Zone | null) => {
        set((state) => {
          switch (action) {
            case 'created':
              if (!data) return state;
              return { zones: [...state.zones, data] };
            case 'updated':
              if (!data) return state;
              return { zones: state.zones.map((z) => (z.id === id ? data : z)) };
            case 'deleted':
              return { zones: state.zones.filter((z) => z.id !== id) };
            default:
              return state;
          }
        });
      },

      applySyncTable: (action: string, id: string, data: Table | null) => {
        set((state) => {
          switch (action) {
            case 'created':
              if (!data) return state;
              return { tables: [...state.tables, data] };
            case 'updated':
              if (!data) return state;
              return { tables: state.tables.map((t) => (t.id === id ? data : t)) };
            case 'deleted':
              return { tables: state.tables.filter((t) => t.id !== id) };
            default:
              return state;
          }
        });
      },

      setDataVersion: (version: number) => set({ dataVersion: version }),

      setIsLoaded: (loaded: boolean) => set({ isLoaded: loaded }),

      performanceMode: false,
      setPerformanceMode: (enabled) => set({ performanceMode: enabled }),
      lastSelectedCategory: '',
      setLastSelectedCategory: (category) => set({ lastSelectedCategory: category }),

      formData: { ...initialFormData },
      formInitialData: { ...initialFormData },
      isFormDirty: false,
      formErrors: {},
  setFormField: (field, value) => {
    const state = get();
    const nextFormData = { ...state.formData, [field]: value };
    const entity = state.modal.entity;
    // Validation
    const errors = validateSettingsForm(entity, nextFormData);
    // Dirty compare by entity
    const isDirty = computeIsDirty(entity, nextFormData, state.formInitialData);
    set({
      formData: nextFormData,
      isFormDirty: isDirty,
      formErrors: errors,
    });
  },
  setFormData: (data) => {
    const state = get();
    const nextFormData = { ...state.formData, ...data };
    const entity = state.modal.entity;
    // Validation
    const errors = validateSettingsForm(entity, nextFormData);
    // Dirty compare by entity
    const isDirty = computeIsDirty(entity, nextFormData, state.formInitialData);
    set({
      formData: nextFormData,
      isFormDirty: isDirty,
      formErrors: errors,
    });
  },
  resetFormData: () => {
    const { formInitialData, modal } = get();
    const nextFormData = { ...formInitialData };
    set({
      formData: nextFormData,
      isFormDirty: false,
      formErrors: validateSettingsForm(modal.entity, nextFormData),
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
    
    // Validate
    const errors = validateSettingsForm(modal.entity, nextFormData);
    // Re-compute dirty state based on the new baseline
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
      lastSelectedCategory: state.lastSelectedCategory
    }),
  }
));

// ============ Granular Selectors ============

export const useSettingsCategory = () =>
  useSettingsStore((state) => state.activeCategory);

export const useSettingsZones = () =>
  useSettingsStore(
    useShallow((state) => ({
      zones: state.zones,
      loading: state.zonesLoading,
      setZones: state.setZones,
      setLoading: state.setZonesLoading,
    }))
  );

export const useSettingsTables = () =>
  useSettingsStore(
    useShallow((state) => ({
      tables: state.tables,
      loading: state.tablesLoading,
      zoneFilter: state.selectedZoneFilter,
      page: state.tablesPage,
      total: state.tablesTotal,
      setTables: state.setTables,
      setLoading: state.setTablesLoading,
      setZoneFilter: state.setSelectedZoneFilter,
      setPagination: state.setTablesPagination,
    }))
  );

export const useSettingsCategories = () =>
  useSettingsStore(
    useShallow((state) => ({
      categories: state.categories,
      loading: state.categoriesLoading,
      setCategories: state.setCategories,
      setLoading: state.setCategoriesLoading,
    }))
  );

export const useSettingsProducts = () =>
  useSettingsStore(
    useShallow((state) => ({
      products: state.products,
      loading: state.productsLoading,
      categoryFilter: state.productCategoryFilter,
      page: state.productsPage,
      total: state.productsTotal,
      setProducts: state.setProducts,
      setLoading: state.setProductsLoading,
      setCategoryFilter: state.setProductCategoryFilter,
      setPagination: state.setProductsPagination,
      updateProductInList: state.updateProductInList,
      removeProductFromList: state.removeProductFromList,
    }))
  );

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

export const useSettingsActions = () =>
  useSettingsStore(
    useShallow((state) => ({
      setActiveCategory: state.setActiveCategory,
      setZones: state.setZones,
      setTables: state.setTables,
      setCategories: state.setCategories,
      setProducts: state.setProducts,
      refreshData: state.refreshData,
      refreshProductsOnly: state.refreshProductsOnly,
      setLastSelectedCategory: state.setLastSelectedCategory,
    }))
  );

export const useDataVersion = () => useSettingsStore((state) => state.dataVersion);

// ============ Validation & Dirty Helpers ============
function validateSettingsForm(entity: ModalEntity, formData: SettingsStore['formData']): Record<string, string | undefined> {
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
    if (formData.externalId === undefined || formData.externalId === null) errors.externalId = 'settings.errors.product.externalIdRequired';
  } else if (entity === 'CATEGORY') {
    if (!formData.name?.trim()) errors.name = 'settings.errors.category.nameRequired';
  }
  return errors;
}

function computeIsDirty(entity: ModalEntity, next: SettingsStore['formData'], initial: SettingsStore['formData']): boolean {
  const pick = (o: SettingsStore['formData'], keys: (keyof SettingsStore['formData'])[]) =>
    keys.map((k) => o[k]);
  if (entity === 'TABLE') {
    const keys: (keyof SettingsStore['formData'])[] = ['name', 'zoneId', 'capacity'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'ZONE') {
    const keys: (keyof SettingsStore['formData'])[] = ['name', 'surchargeType', 'surchargeAmount'];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'PRODUCT') {
    const keys: (keyof SettingsStore['formData'])[] = [
      'name',
      'categoryId',
      'price',
      'externalId',
      'receiptName',
      'image',
      'taxRate',
      'sortOrder',
      'kitchenPrinterId',
      'kitchenPrintName',
      'isKitchenPrintEnabled',
      'isLabelPrintEnabled',
      'hasMultiSpec',
      'selectedAttributeIds',
      'attributeDefaultOptions',
    ];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  } else if (entity === 'CATEGORY') {
    const keys: (keyof SettingsStore['formData'])[] = [
      'name',
      'kitchenPrinterId',
      'isKitchenPrintEnabled',
      'isLabelPrintEnabled',
      'selectedAttributeIds',
      'attributeDefaultOptions',
    ];
    return JSON.stringify(pick(next, keys)) !== JSON.stringify(pick(initial, keys));
  }
  return false;
}
