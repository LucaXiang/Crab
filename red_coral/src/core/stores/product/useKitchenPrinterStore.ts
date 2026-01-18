import { create } from 'zustand';
import { KitchenPrinter } from '@/core/domain/types';
import { createApiClient } from '@/infrastructure/api';
import { logger } from '@/utils/logger';

const api = createApiClient();

interface KitchenPrinterStore {
  // State
  kitchenPrinters: KitchenPrinter[];
  isLoading: boolean;
  error: string | null;

  // Actions
  loadKitchenPrinters: () => Promise<void>;
  createKitchenPrinter: (params: { name: string; connectionType: string; connectionInfo: string; printerName?: string; description?: string }) => Promise<void>;
  updateKitchenPrinter: (params: { id: string; name?: string; connectionType?: string; connectionInfo?: string; printerName?: string; description?: string }) => Promise<void>;
  deleteKitchenPrinter: (id: string) => Promise<void>;
  getKitchenPrinter: (id: string) => KitchenPrinter | undefined;
}

export const useKitchenPrinterStore = create<KitchenPrinterStore>((set, get) => ({
  kitchenPrinters: [],
  isLoading: false,
  error: null,

  loadKitchenPrinters: async () => {
    set({ isLoading: true, error: null });
    try {
      const resp = await api.listPrinters();
      const printers = resp.data?.printers || [];
      set({ kitchenPrinters: printers, isLoading: false });
    } catch (error) {
      logger.error('Failed to load kitchen printers', error);
      set({ error: String(error), isLoading: false });
    }
  },

  createKitchenPrinter: async (params) => {
    set({ isLoading: true, error: null });
    try {
      await api.createPrinter({
        name: params.name,
        printer_name: params.printerName || params.name,
        description: params.description || '',
      });
      await get().loadKitchenPrinters();
    } catch (error) {
      logger.error('Failed to create kitchen printer', error);
      set({ error: String(error), isLoading: false });
      throw error;
    }
  },

  updateKitchenPrinter: async (params) => {
    set({ isLoading: true, error: null });
    try {
      // API expects number ID - convert string to number for legacy API
      await api.updatePrinter(parseInt(params.id, 10), {
        name: params.name || '',
        printer_name: params.printerName || '',
        description: params.description || '',
      });
      await get().loadKitchenPrinters();
    } catch (error) {
      logger.error('Failed to update kitchen printer', error);
      set({ error: String(error), isLoading: false });
      throw error;
    }
  },

  deleteKitchenPrinter: async (id) => {
    set({ isLoading: true, error: null });
    try {
      // API expects number ID - convert string to number for legacy API
      await api.deletePrinter(parseInt(id, 10));
      await get().loadKitchenPrinters();
    } catch (error) {
      logger.error('Failed to delete kitchen printer', error);
      set({ error: String(error), isLoading: false });
      throw error;
    }
  },

  getKitchenPrinter: (id) => {
    return get().kitchenPrinters.find((p) => p.id === id);
  },
}));
