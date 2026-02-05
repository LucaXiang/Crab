import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { createTauriClient } from '@/infrastructure/api';
import type { Attribute, AttributeOption } from '@/core/domain/types/api';
import type { SyncPayload } from '@/core/stores/factory/createResourceStore';
import { useProductStore } from '@/features/product';

const getApi = () => createTauriClient();

/** 属性变更后级联刷新 product store（ProductFull 内嵌了完整属性数据） */
function cascadeRefreshProducts() {
  const productStore = useProductStore.getState();
  if (productStore.isLoaded) {
    productStore.fetchAll(true);
  }
}

// Extended option type with index for UI purposes
interface AttributeOptionWithIndex extends AttributeOption {
  index: number;
  attributeId: string;
}

type AttributeEntity = Attribute & { id: string };

interface AttributeStore {
  // State
  items: AttributeEntity[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;
  lastVersion: number;

  // Options state
  options: Map<string, AttributeOptionWithIndex[]>;
  selectedAttributeId: string | null;

  // Core actions
  fetchAll: (force?: boolean) => Promise<void>;
  applySync: (payload: SyncPayload<AttributeEntity>) => void;
  getById: (id: string) => AttributeEntity | undefined;
  clear: () => void;

  // UI actions
  setSelectedAttributeId: (id: string | null) => void;

  // Options actions
  getOptionsByAttributeId: (attributeId: string) => AttributeOptionWithIndex[];
  loadOptions: (attributeId: string) => Promise<void>;

  // CRUD operations
  createAttribute: (params: {
    name: string;
    is_multi_select?: boolean;
    max_selections?: number | null;
    display_order?: number;
    show_on_receipt?: boolean;
    receipt_name?: string;
    show_on_kitchen_print?: boolean;
    kitchen_print_name?: string;
  }) => Promise<void>;
  updateAttribute: (params: {
    id: string;
    name?: string;
    is_multi_select?: boolean;
    max_selections?: number | null;
    default_option_indices?: number[] | null;
    display_order?: number;
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
    receipt_name?: string;
    kitchen_print_name?: string;
    enable_quantity?: boolean;
    max_quantity?: number | null;
  }) => Promise<void>;
  updateOption: (params: {
    attributeId: string;
    index: number;
    name?: string;
    value_code?: string;
    price_modifier?: number;
    is_default?: boolean;
    display_order?: number;
    receipt_name?: string;
    kitchen_print_name?: string;
    enable_quantity?: boolean;
    max_quantity?: number | null;
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
  lastVersion: 0,
  options: new Map(),
  selectedAttributeId: null,

  // Core actions
  fetchAll: async (force = false) => {
    const state = get();
    if (state.isLoading) return;
    if (state.isLoaded && !force) return;

    set({ isLoading: true, error: null });
    try {
      const attributes = await getApi().listAttributes() as AttributeEntity[];
      set({ items: attributes ?? [], isLoading: false, isLoaded: true });
    } catch (e: unknown) {
      const errorMsg = (e instanceof Error ? e.message : '') || 'Failed to fetch attributes';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] attribute: fetch failed -', errorMsg);
    }
  },

  applySync: (payload: SyncPayload<AttributeEntity>) => {
    const state = get();
    if (!state.isLoaded) return;

    const { id, version, action, data } = payload;

    if (state.lastVersion > 0 && version <= state.lastVersion) return;

    if (state.lastVersion > 0 && version > state.lastVersion + 1) {
      if (!state.isLoading) get().fetchAll(true);
      return;
    }

    switch (action) {
      case 'created':
        if (data) {
          const exists = state.items.some((item) => item.id === id);
          if (exists) {
            set((s) => ({
              items: s.items.map((item) => (item.id === id ? data : item)),
              lastVersion: version,
            }));
          } else {
            set((s) => ({
              items: [...s.items, data],
              lastVersion: version,
            }));
          }
        }
        break;
      case 'updated':
        if (data) {
          set((s) => ({
            items: s.items.map((item) => (item.id === id ? data : item)),
            lastVersion: version,
          }));
          // 同步更新 options 缓存（attribute 内嵌 options）
          if (data.options) {
            const opts: AttributeOptionWithIndex[] = data.options.map((opt, index) => ({
              ...opt,
              index,
              attributeId: id,
            }));
            set((s) => {
              const newOptions = new Map(s.options);
              newOptions.set(id, opts);
              return { options: newOptions };
            });
          }
        }
        break;
      case 'deleted':
        set((s) => {
          const newOptions = new Map(s.options);
          newOptions.delete(id);
          return {
            items: s.items.filter((item) => item.id !== id),
            options: newOptions,
            lastVersion: version,
          };
        });
        break;
    }
  },

  getById: (id) => get().items.find((item) => item.id === id),

  clear: () => set({ items: [], isLoaded: false, error: null, lastVersion: 0, options: new Map() }),

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
      const template = await getApi().getAttribute(attributeId);
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
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Unknown error';
      console.error('[Store] attribute: loadOptions failed -', msg);
      set({ error: msg, isLoading: false });
    }
  },

  // CRUD operations
  createAttribute: async (params) => {
    try {
      await getApi().createAttribute(params);
      await get().fetchAll(true);
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] createAttribute failed:', e);
      throw e;
    }
  },
  updateAttribute: async (params) => {
    try {
      const { id, ...data } = params;
      await getApi().updateAttribute(id, data);
      await get().fetchAll(true);
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] updateAttribute failed:', e);
      throw e;
    }
  },
  deleteAttribute: async (id) => {
    try {
      await getApi().deleteAttribute(id);
      await get().fetchAll(true);
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] deleteAttribute failed:', e);
      throw e;
    }
  },
  createOption: async (params) => {
    try {
      const { attributeId, ...data } = params;
      const template = await getApi().addAttributeOption(attributeId, data);
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
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] createOption failed:', e);
      throw e;
    }
  },
  updateOption: async (params) => {
    try {
      const { attributeId, index, ...data } = params;
      const template = await getApi().updateAttributeOption(attributeId, index, data);
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
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] updateOption failed:', e);
      throw e;
    }
  },
  deleteOption: async (attributeId, index) => {
    try {
      const template = await getApi().deleteAttributeOption(attributeId, index);
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
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] deleteOption failed:', e);
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
      await getApi().updateAttribute(attributeId, { options: reorderedOptions });

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
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] reorderOptions failed:', e);
      throw e;
    }
  },
  bindProductAttribute: async (params) => {
    try {
      await getApi().bindProductAttribute({
        product_id: params.product_id,
        attribute_id: params.attribute_id,
        is_required: params.is_required,
        display_order: params.display_order,
      });
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] bindProductAttribute failed:', e);
      throw e;
    }
  },
  unbindProductAttribute: async (bindingId) => {
    try {
      await getApi().unbindProductAttribute(bindingId);
      cascadeRefreshProducts();
    } catch (e: unknown) {
      console.error('[Store] unbindProductAttribute failed:', e);
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
      fetchAll: state.fetchAll,
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
