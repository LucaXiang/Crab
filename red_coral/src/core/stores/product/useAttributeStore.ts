import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import type { Attribute, AttributeOption } from '@/infrastructure/api/types';

// Extended option type with index for UI purposes
interface AttributeOptionWithIndex extends AttributeOption {
  index: number;
  attributeId: string;
}

interface AttributeStore {
  // State - using API types directly
  attributes: Attribute[];
  options: Map<string, AttributeOptionWithIndex[]>;
  isLoading: boolean;
  error: string | null;
  selectedAttributeId: string | null;

  // Actions
  setSelectedAttributeId: (id: string | null) => void;

  // All methods marked as TODO
  loadAttributes: () => Promise<void>;
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
  loadOptions: (attributeId: string) => Promise<void>;
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

  // Helper method to get options by attribute ID
  getOptionsByAttributeId: (attributeId: string) => AttributeOptionWithIndex[];
}

// Create store with helper functions outside to avoid recreation
const createAttributeStore = (set: any, get: any): AttributeStore => ({
  attributes: [],
  options: new Map(),
  isLoading: false,
  error: null,
  selectedAttributeId: null,

  setSelectedAttributeId: (id: string | null) => {
    set({ selectedAttributeId: id });
  },

  loadAttributes: async () => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  createAttribute: async (_params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  updateAttribute: async (_params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  deleteAttribute: async (_id: string) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  loadOptions: async (_attributeId: string) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  createOption: async (_params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  updateOption: async (_params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  deleteOption: async (_attributeId: string, _index: number) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  reorderOptions: async (_attributeId: string, _ids: string[]) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  bindProductAttribute: async (_params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  unbindProductAttribute: async (_productId: string, _attributeId: string) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  getOptionsByAttributeId: (attributeId: string): AttributeOptionWithIndex[] => {
    return get().options.get(attributeId) || [];
  },
});

// Create store instance
const attributeStore = create<AttributeStore>(createAttributeStore);

// Helper functions that don't change between renders
const getAttributeByIdHelper = (id: string) => {
  const attrs = attributeStore.getState().attributes;
  return attrs.find((attr) => String(attr.id) === id);
};

export const useAttributeStore = attributeStore;

// Selectors
export const useAttributes = () =>
  useAttributeStore((state) => state.attributes);

export const useAttributeLoading = () =>
  useAttributeStore((state) => state.isLoading);

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
  getAttributeById: getAttributeByIdHelper,
  getOptionsByAttributeId: (attributeId: string) => {
    const opts = attributeStore.getState().options.get(attributeId) || [];
    return opts;
  },
};

export const useAttributeHelpers = () => attributeHelpers;
