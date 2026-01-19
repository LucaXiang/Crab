import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { createTauriClient } from '@/infrastructure/api';
import type { Attribute, AttributeOption } from '@/core/domain/types/api';

const api = createTauriClient();

// Extended option type with index for UI purposes
interface AttributeOptionWithIndex extends AttributeOption {
  index: number;
  attributeId: string;
}

type AttributeEntity = Attribute & { id: string };

interface AttributeStore {
  // State (new architecture)
  items: AttributeEntity[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;

  // Options state
  options: Map<string, AttributeOptionWithIndex[]>;
  selectedAttributeId: string | null;

  // Core actions (new architecture)
  fetchAll: () => Promise<void>;
  applySync: () => void;
  getById: (id: string) => AttributeEntity | undefined;
  clear: () => void;

  // UI actions
  setSelectedAttributeId: (id: string | null) => void;

  // Options actions
  getOptionsByAttributeId: (attributeId: string) => AttributeOptionWithIndex[];
  loadOptions: (attributeId: string) => Promise<void>;

  // Legacy compatibility - attributes alias
  attributes: AttributeEntity[];
  loadAttributes: () => Promise<void>;

  // CRUD stubs (not implemented - use HTTP API directly)
  createAttribute: (params: {
    name: string;
    attr_type: string;
    display_order?: number;
    is_active?: boolean;
    show_on_receipt?: boolean;
    receipt_name?: string;
    kitchen_printer?: string;
  }) => Promise<void>;
  updateAttribute: (params: {
    id: string;
    name?: string;
    attr_type?: string;
    display_order?: number;
    is_active?: boolean;
    show_on_receipt?: boolean;
    receipt_name?: string;
    kitchen_printer?: string;
  }) => Promise<void>;
  deleteAttribute: (id: string) => Promise<void>;
  createOption: (params: {
    attributeId: string;
    name: string;
    value_code?: string;
    price_modifier?: number;
    is_default?: boolean;
    display_order?: number;
    is_active?: boolean;
    receipt_name?: string;
  }) => Promise<void>;
  updateOption: (params: {
    attributeId: string;
    index: number;
    name?: string;
    value_code?: string;
    price_modifier?: number;
    is_default?: boolean;
    display_order?: number;
    is_active?: boolean;
    receipt_name?: string;
  }) => Promise<void>;
  deleteOption: (attributeId: string, index: number) => Promise<void>;
  reorderOptions: (attributeId: string, ids: string[]) => Promise<void>;
  bindProductAttribute: (params: {
    productId: string;
    attributeId: string;
    is_required?: boolean;
    display_order?: number;
    default_option_idx?: number;
  }) => Promise<void>;
  unbindProductAttribute: (productId: string, attributeId: string) => Promise<void>;
}

export const useAttributeStore = create<AttributeStore>((set, get) => ({
  // State
  items: [],
  isLoading: false,
  isLoaded: false,
  error: null,
  options: new Map(),
  selectedAttributeId: null,

  // Core actions
  fetchAll: async () => {
    set({ isLoading: true, error: null });
    try {
      const response = await api.listAttributeTemplates();
      // Handle both formats: direct array or { data: { templates: [...] } }
      const attributes = Array.isArray(response)
        ? (response as AttributeEntity[])
        : ((response.data?.templates || []) as AttributeEntity[]);
      set({ items: attributes, isLoading: false, isLoaded: true });
    } catch (e: any) {
      const errorMsg = e.message || 'Failed to fetch attributes';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] attribute: fetch failed -', errorMsg);
    }
  },

  applySync: () => {
    if (get().isLoaded) {
      get().fetchAll();
    }
  },

  getById: (id) => get().items.find((item) => item.id === id),

  clear: () => set({ items: [], isLoaded: false, error: null, options: new Map() }),

  // UI actions
  setSelectedAttributeId: (id) => set({ selectedAttributeId: id }),

  // Options actions
  getOptionsByAttributeId: (attributeId) => {
    return get().options.get(attributeId) || [];
  },

  loadOptions: async (attributeId) => {
    set({ isLoading: true, error: null });
    try {
      // Options are embedded in Attribute, fetch the attribute to get its options
      const response = await api.getAttributeTemplate(attributeId);
      const opts = (response.data?.template?.options || []) as AttributeOption[];
      const optionsWithIndex: AttributeOptionWithIndex[] = opts.map((opt, index) => ({
        ...opt,
        index,
        attributeId,
      }));
      set((state) => {
        const newOptions = new Map(state.options);
        newOptions.set(attributeId, optionsWithIndex);
        return { options: newOptions, isLoading: false };
      });
    } catch (e: any) {
      console.error('[Store] attribute: loadOptions failed -', e.message);
      set({ error: e.message, isLoading: false });
    }
  },

  // Legacy compatibility
  get attributes() {
    return get().items;
  },

  loadAttributes: async () => {
    await get().fetchAll();
  },

  // CRUD stubs - use HTTP API directly instead
  createAttribute: async (_params) => {
    console.warn('[Store] createAttribute not implemented - use HTTP API');
  },
  updateAttribute: async (_params) => {
    console.warn('[Store] updateAttribute not implemented - use HTTP API');
  },
  deleteAttribute: async (_id) => {
    console.warn('[Store] deleteAttribute not implemented - use HTTP API');
  },
  createOption: async (_params) => {
    console.warn('[Store] createOption not implemented - use HTTP API');
  },
  updateOption: async (_params) => {
    console.warn('[Store] updateOption not implemented - use HTTP API');
  },
  deleteOption: async (_attributeId, _index) => {
    console.warn('[Store] deleteOption not implemented - use HTTP API');
  },
  reorderOptions: async (_attributeId, _ids) => {
    console.warn('[Store] reorderOptions not implemented - use HTTP API');
  },
  bindProductAttribute: async (_params) => {
    console.warn('[Store] bindProductAttribute not implemented - use HTTP API');
  },
  unbindProductAttribute: async (_productId, _attributeId) => {
    console.warn('[Store] unbindProductAttribute not implemented - use HTTP API');
  },
}));

// Convenience hooks
export const useAttributes = () => useAttributeStore((state) => state.items);
export const useAttributesLoading = () => useAttributeStore((state) => state.isLoading);
export const useAttributeById = (id: string) =>
  useAttributeStore((state) => state.items.find((a) => a.id === id));

// Action hooks
export const useAttributeActions = () =>
  useAttributeStore(
    useShallow((state) => ({
      setSelectedAttributeId: state.setSelectedAttributeId,
      loadAttributes: state.loadAttributes,
      createAttribute: state.createAttribute,
      updateAttribute: state.updateAttribute,
      deleteAttribute: state.deleteAttribute,
      loadOptions: state.loadOptions,
      createOption: state.createOption,
      updateOption: state.updateOption,
      deleteOption: state.deleteOption,
      reorderOptions: state.reorderOptions,
      bindProductAttribute: state.bindProductAttribute,
      unbindProductAttribute: state.unbindProductAttribute,
    }))
  );

export const useOptionActions = () =>
  useAttributeStore(
    useShallow((state) => ({
      loadOptions: state.loadOptions,
      createOption: state.createOption,
      updateOption: state.updateOption,
      deleteOption: state.deleteOption,
    }))
  );

// Stable helper object - same reference every render
export const attributeHelpers = {
  getAttributeById: (id: string) => {
    return useAttributeStore.getState().items.find((attr) => String(attr.id) === id);
  },
  getOptionsByAttributeId: (attributeId: string) => {
    return useAttributeStore.getState().options.get(attributeId) || [];
  },
};

export const useAttributeHelpers = () => attributeHelpers;
