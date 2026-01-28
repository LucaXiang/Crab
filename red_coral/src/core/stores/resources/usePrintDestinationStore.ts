import { create } from 'zustand';
import { createTauriClient } from '@/infrastructure/api';
import type { PrintDestination, PrintDestinationCreate, PrintDestinationUpdate } from '@/core/domain/types/api';

const api = createTauriClient();

type PrintDestinationEntity = PrintDestination & { id: string };

interface PrintDestinationStore {
  // State
  items: PrintDestinationEntity[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;

  // Core actions (new architecture)
  fetchAll: (force?: boolean) => Promise<void>;
  applySync: () => void;
  getById: (id: string) => PrintDestinationEntity | undefined;
  clear: () => void;

  // CRUD actions
  create: (data: PrintDestinationCreate) => Promise<PrintDestinationEntity>;
  update: (id: string, data: PrintDestinationUpdate) => Promise<PrintDestinationEntity>;
  remove: (id: string) => Promise<void>;

  // Optimistic update helpers
  optimisticAdd: (item: PrintDestinationEntity) => void;
  optimisticUpdate: (id: string, updater: (item: PrintDestinationEntity) => PrintDestinationEntity) => void;
  optimisticRemove: (id: string) => void;

}

export const usePrintDestinationStore = create<PrintDestinationStore>((set, get) => ({
  // State
  items: [],
  isLoading: false,
  isLoaded: false,
  error: null,

  // Core actions
  fetchAll: async (force = false) => {
    const state = get();
    if (state.isLoading) return;
    if (state.isLoaded && !force) return;

    set({ isLoading: true, error: null });
    try {
      const destinations = await api.listPrintDestinations() as PrintDestinationEntity[];
      const safeDestinations = destinations ?? [];
      safeDestinations.sort((a, b) => a.name.localeCompare(b.name));
      set({ items: safeDestinations, isLoading: false, isLoaded: true });
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to fetch print destinations';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] print_destination: fetch failed -', errorMsg);
    }
  },

  applySync: () => {
    if (get().isLoaded) {
      get().fetchAll(true);
    }
  },

  getById: (id) => get().items.find((item) => item.id === id),

  clear: () => set({ items: [], isLoaded: false, error: null }),

  // CRUD actions
  create: async (data) => {
    set({ isLoading: true, error: null });
    try {
      const newDestination = await api.createPrintDestination(data) as PrintDestinationEntity;
      await get().fetchAll(true);
      return newDestination || get().items[get().items.length - 1];
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to create print destination';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] print_destination: create failed -', errorMsg);
      throw e;
    }
  },

  update: async (id, data) => {
    set({ isLoading: true, error: null });
    try {
      await api.updatePrintDestination(id, data);
      await get().fetchAll(true);
      const updated = get().items.find((p) => p.id === id);
      if (!updated) {
        throw new Error(`PrintDestination ${id} not found after update`);
      }
      return updated;
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to update print destination';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] print_destination: update failed -', errorMsg);
      throw e;
    }
  },

  remove: async (id) => {
    set({ isLoading: true, error: null });
    try {
      await api.deletePrintDestination(id);
      set((state) => ({
        items: state.items.filter((item) => item.id !== id),
        isLoading: false,
      }));
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to delete print destination';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] print_destination: remove failed -', errorMsg);
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

}));

// Convenience hooks
export const usePrintDestinations = () => usePrintDestinationStore((state) => state.items);
export const usePrintDestinationsLoading = () => usePrintDestinationStore((state) => state.isLoading);
export const usePrintDestinationById = (id: string) =>
  usePrintDestinationStore((state) => state.items.find((p) => p.id === id));

// CRUD action hooks
export const usePrintDestinationActions = () => ({
  create: usePrintDestinationStore.getState().create,
  update: usePrintDestinationStore.getState().update,
  remove: usePrintDestinationStore.getState().remove,
  fetchAll: usePrintDestinationStore.getState().fetchAll,
});
