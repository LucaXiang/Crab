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
  fetchAll: (force?: boolean) => Promise<void>;
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

  // CRUD operations
  createAttribute: (params: {
    name: string;
    scope?: 'global' | 'inherited';
    is_multi_select?: boolean;
    max_selections?: number | null;
    display_order?: number;
    is_active?: boolean;
    show_on_receipt?: boolean;
    receipt_name?: string;
    show_on_kitchen_print?: boolean;
    kitchen_print_name?: string;
  }) => Promise<void>;
  updateAttribute: (params: {
    id: string;
    name?: string;
    scope?: 'global' | 'inherited';
    is_multi_select?: boolean;
    max_selections?: number | null;
    display_order?: number;
    is_active?: boolean;
    show_on_receipt?: boolean;
    receipt_name?: string;
    show_on_kitchen_print?: boolean;
    kitchen_print_name?: string;
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
    kitchen_print_name?: string;
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
    kitchen_print_name?: string;
  }) => Promise<void>;
  deleteOption: (attributeId: string, index: number) => Promise<void>;
  reorderOptions: (attributeId: string, ids: string[]) => Promise<void>;
  bindProductAttribute: (params: {
    product_id: string;
    attribute_id: string;
    is_required?: boolean;
    display_order?: number;
  }) => Promise<void>;
  unbindProductAttribute: (bindingId: string) => Promise<void>;
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
  fetchAll: async (force = false) => {
    // Guard: skip if already loading, or already loaded (unless forced)
    const state = get();
    if (state.isLoading) return;
    if (state.isLoaded && !force) return;

    set({ isLoading: true, error: null });
    try {
      const attributes = await api.listAttributeTemplates() as AttributeEntity[];
      set({ items: attributes ?? [], isLoading: false, isLoaded: true });
    } catch (e: any) {
      const errorMsg = e.message || 'Failed to fetch attributes';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] attribute: fetch failed -', errorMsg);
    }
  },

  applySync: () => {
    if (get().isLoaded) {
      get().fetchAll(true);  // Force refresh on sync
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
      const template = await api.getAttributeTemplate(attributeId);
      const opts = (template.options || []) as AttributeOption[];
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

  // CRUD operations
  createAttribute: async (params) => {
    try {
      await api.createAttribute(params);
      await get().fetchAll(true);
    } catch (e: any) {
      console.error('[Store] createAttribute failed:', e.message);
      throw e;
    }
  },
  updateAttribute: async (params) => {
    try {
      const { id, ...data } = params;
      await api.updateAttribute(id, data);
      await get().fetchAll(true);
    } catch (e: any) {
      console.error('[Store] updateAttribute failed:', e.message);
      throw e;
    }
  },
  deleteAttribute: async (id) => {
    try {
      await api.deleteAttribute(id);
      await get().fetchAll(true);
    } catch (e: any) {
      console.error('[Store] deleteAttribute failed:', e.message);
      throw e;
    }
  },
  createOption: async (params) => {
    try {
      const { attributeId, ...data } = params;
      const template = await api.addAttributeOption(attributeId, data);
      // Update local options cache
      const opts = (template.options || []).map((opt, index) => ({
        ...opt,
        index,
        attributeId,
      }));
      set((state) => {
        const newOptions = new Map(state.options);
        newOptions.set(attributeId, opts);
        return { options: newOptions };
      });
      await get().fetchAll(true);
    } catch (e: any) {
      console.error('[Store] createOption failed:', e.message);
      throw e;
    }
  },
  updateOption: async (params) => {
    try {
      const { attributeId, index, ...data } = params;
      const template = await api.updateAttributeOption(attributeId, index, data);
      // Update local options cache
      const opts = (template.options || []).map((opt, idx) => ({
        ...opt,
        index: idx,
        attributeId,
      }));
      set((state) => {
        const newOptions = new Map(state.options);
        newOptions.set(attributeId, opts);
        return { options: newOptions };
      });
      await get().fetchAll(true);
    } catch (e: any) {
      console.error('[Store] updateOption failed:', e.message);
      throw e;
    }
  },
  deleteOption: async (attributeId, index) => {
    try {
      const template = await api.deleteAttributeOption(attributeId, index);
      // Update local options cache
      const opts = (template.options || []).map((opt, idx) => ({
        ...opt,
        index: idx,
        attributeId,
      }));
      set((state) => {
        const newOptions = new Map(state.options);
        newOptions.set(attributeId, opts);
        return { options: newOptions };
      });
      await get().fetchAll(true);
    } catch (e: any) {
      console.error('[Store] deleteOption failed:', e.message);
      throw e;
    }
  },
  reorderOptions: async (attributeId, newOrder) => {
    try {
      // Get current attribute with options
      const attr = get().items.find(a => a.id === attributeId);
      if (!attr?.options) return;

      // Reorder options array based on newOrder indices
      const reorderedOptions = newOrder.map((idxStr, newIdx) => {
        const oldIdx = parseInt(idxStr, 10);
        const opt = attr.options![oldIdx];
        return { ...opt, display_order: newIdx };
      });

      // Single API call to update all options
      await api.updateAttribute(attributeId, { options: reorderedOptions });

      // Update local state directly (no extra API calls)
      set((state) => {
        const newItems = state.items.map(item =>
          item.id === attributeId ? { ...item, options: reorderedOptions } : item
        );
        const newOptionsMap = new Map(state.options);
        newOptionsMap.set(attributeId, reorderedOptions.map((opt, index) => ({
          ...opt,
          index,
          attributeId,
        })));
        return { items: newItems, options: newOptionsMap };
      });
    } catch (e: any) {
      console.error('[Store] reorderOptions failed:', e.message);
      throw e;
    }
  },
  bindProductAttribute: async (params) => {
    try {
      // Note: default_option_idx is now on the Attribute itself, not on the binding
      await api.bindProductAttribute({
        product_id: params.product_id,
        attribute_id: params.attribute_id,
        is_required: params.is_required,
        display_order: params.display_order,
      });
    } catch (e: any) {
      console.error('[Store] bindProductAttribute failed:', e.message);
      throw e;
    }
  },
  unbindProductAttribute: async (bindingId) => {
    try {
      await api.unbindProductAttribute(bindingId);
    } catch (e: any) {
      console.error('[Store] unbindProductAttribute failed:', e.message);
      throw e;
    }
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
