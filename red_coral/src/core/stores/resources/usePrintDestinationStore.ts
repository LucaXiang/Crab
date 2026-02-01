import { create } from 'zustand';
import { createTauriClient } from '@/infrastructure/api';
import type { PrintDestination, PrintDestinationCreate, PrintDestinationUpdate } from '@/core/domain/types/api';
import type { SyncPayload } from '../factory/createResourceStore';

const getApi = () => createTauriClient();

type PrintDestinationEntity = PrintDestination & { id: string };

interface PrintDestinationStore {
  // State
  items: PrintDestinationEntity[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;
  lastVersion: number;

  // Core actions
  fetchAll: (force?: boolean) => Promise<void>;
  applySync: (payload: SyncPayload<PrintDestinationEntity>) => void;
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
  lastVersion: 0,

  // Core actions
  fetchAll: async (force = false) => {
    const state = get();
    if (state.isLoading) return;
    if (state.isLoaded && !force) return;

    set({ isLoading: true, error: null });
    try {
      const destinations = await getApi().listPrintDestinations() as PrintDestinationEntity[];
      const safeDestinations = destinations ?? [];
      safeDestinations.sort((a, b) => a.name.localeCompare(b.name));
      set({ items: safeDestinations, isLoading: false, isLoaded: true });
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to fetch print destinations';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] print_destination: fetch failed -', errorMsg);
    }
  },

  applySync: (payload: SyncPayload<PrintDestinationEntity>) => {
    const state = get();
    if (!state.isLoaded) return;

    const { id, version, action, data } = payload;

    // Skip duplicate
    if (state.lastVersion > 0 && version <= state.lastVersion) {
      return;
    }

    // Gap detected: full refresh
    if (state.lastVersion > 0 && version > state.lastVersion + 1) {
      if (state.isLoaded && !state.isLoading) {
        get().fetchAll(true);
      }
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
        }
        break;
      case 'deleted':
        set((s) => ({
          items: s.items.filter((item) => item.id !== id),
          lastVersion: version,
        }));
        break;
    }
  },

  getById: (id) => get().items.find((item) => item.id === id),

  clear: () => set({ items: [], isLoaded: false, error: null, lastVersion: 0 }),

  // CRUD actions
  create: async (data) => {
    set({ isLoading: true, error: null });
    try {
      const newDestination = await getApi().createPrintDestination(data) as PrintDestinationEntity;
      // 直接更新 items，不依赖 fetchAll（避免 isLoading 互锁）
      set((state) => ({
        items: [...state.items, newDestination],
        isLoading: false,
      }));
      return newDestination;
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
      const updated = await getApi().updatePrintDestination(id, data) as PrintDestinationEntity;
      // 直接替换 items 中对应项
      set((state) => ({
        items: state.items.map((item) => (item.id === id ? updated : item)),
        isLoading: false,
      }));
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
      await getApi().deletePrintDestination(id);
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
