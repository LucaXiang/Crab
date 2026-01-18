import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';

interface AttributeStore {
  // State
  attributes: Array<{
    id: number;
    name: string;
    type: string;
    displayOrder: number;
    isActive: boolean;
    showOnReceipt: boolean;
    receiptName?: string;
    kitchenPrinterId?: number;
  }>;
  options: Map<string, Array<{
    id: number;
    attributeId: number;
    name: string;
    receiptName?: string;
    valueCode?: string;
    priceModifier: number;
    isDefault: boolean;
    displayOrder: number;
    isActive: boolean;
  }>>;
  isLoading: boolean;
  error: string | null;
  selectedAttributeId: string | null;

  // Actions
  setSelectedAttributeId: (id: string | null) => void;

  // All methods marked as TODO
  loadAttributes: () => Promise<void>;
  createAttribute: (params: {
    name: string;
    type: string;
    displayOrder?: number;
    isActive?: boolean;
    showOnReceipt?: boolean;
    receiptName?: string;
    kitchenPrinterId?: number;
  }) => Promise<void>;
  updateAttribute: (params: {
    id: number;
    name?: string;
    type?: string;
    displayOrder?: number;
    isActive?: boolean;
    showOnReceipt?: boolean;
    receiptName?: string;
    kitchenPrinterId?: number;
  }) => Promise<void>;
  deleteAttribute: (id: string) => Promise<void>;
  loadOptions: (attributeId: string) => Promise<void>;
  createOption: (params: {
    attributeId: number;
    name: string;
    valueCode: string;
    priceModifier?: number;
    isDefault?: boolean;
    displayOrder?: number;
    isActive?: boolean;
    receiptName?: string;
  }) => Promise<void>;
  updateOption: (params: {
    id: number;
    name?: string;
    valueCode?: string;
    priceModifier?: number;
    isDefault?: boolean;
    displayOrder?: number;
    isActive?: boolean;
    receiptName?: string;
  }) => Promise<void>;
  deleteOption: (id: string) => Promise<void>;
  reorderOptions: (attributeId: string, ids: string[]) => Promise<void>;
  bindProductAttribute: (params: {
    productId: number;
    attributeId: number;
    isRequired?: boolean;
    displayOrder?: number;
    defaultOptionId?: number;
  }) => Promise<void>;
  unbindProductAttribute: (productId: string, attributeId: string) => Promise<void>;

  // Helper method to get options by attribute ID
  getOptionsByAttributeId: (attributeId: number) => Array<{
    id: number;
    attributeId: number;
    name: string;
    receiptName?: string;
    valueCode?: string;
    priceModifier: number;
    isDefault: boolean;
    displayOrder: number;
    isActive: boolean;
  }>;
}

// Create store with helper functions outside to avoid recreation
const createAttributeStore = (set: any, get: any) => ({
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

  createAttribute: async (params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  updateAttribute: async (params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  deleteAttribute: async (id: string) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  loadOptions: async (attributeId: string) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  createOption: async (params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  updateOption: async (params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  deleteOption: async (id: string) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  reorderOptions: async (attributeId: string, ids: string[]) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  bindProductAttribute: async (params: any) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  unbindProductAttribute: async (productId: string, attributeId: string) => {
    set({ isLoading: true, error: null });
    try {
      throw new Error('Not implemented: Use HTTP API instead');
    } catch (error: any) {
      set({ error: error.message, isLoading: false });
    }
  },

  getOptionsByAttributeId: (attributeId: number) => {
    return get().options.get(String(attributeId)) || [];
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
  getOptionsByAttributeId: (attributeId: number) => {
    const opts = attributeStore.getState().options.get(String(attributeId)) || [];
    return opts;
  },
};

export const useAttributeHelpers = () => attributeHelpers;
