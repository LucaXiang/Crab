/**
 * Store Info Store - Fetches and updates store information from API
 *
 * Store info is a singleton per tenant, used for:
 * - Receipt headers
 * - Label printing
 * - Business info display
 */

import { create } from 'zustand';
import { createTauriClient } from '@/infrastructure/api';
import type { StoreInfo, StoreInfoUpdate } from '@/core/domain/types/api';

const api = createTauriClient();

interface StoreInfoState {
  // State
  info: StoreInfo;
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;

  // Actions
  fetchStoreInfo: (force?: boolean) => Promise<void>;
  updateStoreInfo: (data: StoreInfoUpdate) => Promise<StoreInfo>;
  clear: () => void;
}

const defaultStoreInfo: StoreInfo = {
  id: null,
  name: '',
  address: '',
  nif: '',
  logo_url: null,
  phone: null,
  email: null,
  website: null,
  created_at: null,
  updated_at: null,
};

export const useStoreInfoStore = create<StoreInfoState>((set, get) => ({
  // State
  info: defaultStoreInfo,
  isLoading: false,
  isLoaded: false,
  error: null,

  // Actions
  fetchStoreInfo: async (force = false) => {
    const state = get();
    if (state.isLoading) return;
    if (state.isLoaded && !force) return;

    set({ isLoading: true, error: null });
    try {
      const info = await api.getStoreInfo();
      set({ info, isLoading: false, isLoaded: true });
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to fetch store info';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] store_info: fetch failed -', errorMsg);
    }
  },

  updateStoreInfo: async (data: StoreInfoUpdate) => {
    set({ isLoading: true, error: null });
    try {
      const updated = await api.updateStoreInfo(data);
      set({ info: updated, isLoading: false, isLoaded: true });
      return updated;
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to update store info';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] store_info: update failed -', errorMsg);
      throw e;
    }
  },

  clear: () => set({ info: defaultStoreInfo, isLoaded: false, error: null }),
}));

// Convenience hooks
export const useStoreInfo = () => useStoreInfoStore((state) => state.info);
export const useStoreInfoLoading = () => useStoreInfoStore((state) => state.isLoading);
export const useStoreInfoActions = () => ({
  fetch: useStoreInfoStore.getState().fetchStoreInfo,
  update: useStoreInfoStore.getState().updateStoreInfo,
});
