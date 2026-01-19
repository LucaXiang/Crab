import { create } from 'zustand';
import { createTauriClient } from '@/infrastructure/api';
import type { KitchenPrinter } from '@/infrastructure/api/types';

const api = createTauriClient();

// Create input type
interface CreateKitchenPrinterInput {
  name: string;
  printerName?: string;
  description?: string;
}

// Update input type
interface UpdateKitchenPrinterInput {
  name?: string;
  printerName?: string;
  description?: string;
}

type KitchenPrinterEntity = KitchenPrinter & { id: string };

interface KitchenPrinterStore {
  // State
  items: KitchenPrinterEntity[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;

  // Core actions (new architecture)
  fetchAll: () => Promise<void>;
  applySync: () => void;
  getById: (id: string) => KitchenPrinterEntity | undefined;
  clear: () => void;

  // CRUD actions
  create: (data: CreateKitchenPrinterInput) => Promise<KitchenPrinterEntity>;
  update: (id: string, data: UpdateKitchenPrinterInput) => Promise<KitchenPrinterEntity>;
  remove: (id: string) => Promise<void>;

  // Optimistic update helpers
  optimisticAdd: (item: KitchenPrinterEntity) => void;
  optimisticUpdate: (id: string, updater: (item: KitchenPrinterEntity) => KitchenPrinterEntity) => void;
  optimisticRemove: (id: string) => void;

  // Legacy API compatibility
  kitchenPrinters: KitchenPrinterEntity[];
  loadKitchenPrinters: () => Promise<void>;
  createKitchenPrinter: (params: CreateKitchenPrinterInput) => Promise<void>;
  updateKitchenPrinter: (params: { id: string } & UpdateKitchenPrinterInput) => Promise<void>;
  deleteKitchenPrinter: (id: string) => Promise<void>;
  getKitchenPrinter: (id: string) => KitchenPrinterEntity | undefined;
}

export const useKitchenPrinterStore = create<KitchenPrinterStore>((set, get) => ({
  // State
  items: [],
  isLoading: false,
  isLoaded: false,
  error: null,

  // Core actions
  fetchAll: async () => {
    set({ isLoading: true, error: null });
    try {
      const response = await api.listPrinters();
      const printers = (response.data?.printers || []) as KitchenPrinterEntity[];
      set({ items: printers, isLoading: false, isLoaded: true });
    } catch (e: any) {
      const errorMsg = e.message || 'Failed to fetch kitchen printers';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] kitchen_printer: fetch failed -', errorMsg);
    }
  },

  applySync: () => {
    if (get().isLoaded) {
      get().fetchAll();
    }
  },

  getById: (id) => get().items.find((item) => item.id === id),

  clear: () => set({ items: [], isLoaded: false, error: null }),

  // CRUD actions
  create: async (data) => {
    set({ isLoading: true, error: null });
    try {
      const response = await api.createPrinter({
        name: data.name,
        printer_name: data.printerName || data.name,
        description: data.description || '',
      });
      const newPrinter = response.data?.printer as KitchenPrinterEntity;
      // Refresh to get latest data
      await get().fetchAll();
      return newPrinter || get().items[get().items.length - 1];
    } catch (e: any) {
      set({ error: e.message, isLoading: false });
      console.error('[Store] kitchen_printer: create failed -', e.message);
      throw e;
    }
  },

  update: async (id, data) => {
    set({ isLoading: true, error: null });
    try {
      await api.updatePrinter(parseInt(id, 10), {
        name: data.name || '',
        printer_name: data.printerName || '',
        description: data.description || '',
      });
      // Refresh to get latest data
      await get().fetchAll();
      const updated = get().items.find((p) => p.id === id);
      return updated!;
    } catch (e: any) {
      set({ error: e.message, isLoading: false });
      console.error('[Store] kitchen_printer: update failed -', e.message);
      throw e;
    }
  },

  remove: async (id) => {
    set({ isLoading: true, error: null });
    try {
      await api.deletePrinter(parseInt(id, 10));
      set((state) => ({
        items: state.items.filter((item) => item.id !== id),
        isLoading: false,
      }));
    } catch (e: any) {
      set({ error: e.message, isLoading: false });
      console.error('[Store] kitchen_printer: remove failed -', e.message);
      throw e;
    }
  },

  // Optimistic update helpers
  optimisticAdd: (item) => {
    set((state) => ({ items: [...state.items, item] }));
  },

  optimisticUpdate: (id, updater) => {
    set((state) => ({
      items: state.items.map((item) => (item.id === id ? updater(item) : item)),
    }));
  },

  optimisticRemove: (id) => {
    set((state) => ({
      items: state.items.filter((item) => item.id !== id),
    }));
  },

  // Legacy API compatibility (getter)
  get kitchenPrinters() {
    return get().items;
  },

  loadKitchenPrinters: async () => {
    await get().fetchAll();
  },

  createKitchenPrinter: async (params) => {
    await get().create(params);
  },

  updateKitchenPrinter: async (params) => {
    const { id, ...data } = params;
    await get().update(id, data);
  },

  deleteKitchenPrinter: async (id) => {
    await get().remove(id);
  },

  getKitchenPrinter: (id) => {
    return get().getById(id);
  },
}));

// Register in store registry
import { storeRegistry } from './registry';
storeRegistry['kitchen_printer'] = useKitchenPrinterStore;

// Convenience hooks
export const useKitchenPrinters = () => useKitchenPrinterStore((state) => state.items);
export const useKitchenPrintersLoading = () => useKitchenPrinterStore((state) => state.isLoading);
export const useKitchenPrinterById = (id: string) =>
  useKitchenPrinterStore((state) => state.items.find((p) => p.id === id));

// CRUD action hooks
export const useKitchenPrinterActions = () => ({
  create: useKitchenPrinterStore.getState().create,
  update: useKitchenPrinterStore.getState().update,
  remove: useKitchenPrinterStore.getState().remove,
  fetchAll: useKitchenPrinterStore.getState().fetchAll,
});
