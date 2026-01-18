/**
 * Draft Orders Store
 * Manages draft orders (orders saved but not yet finalized)
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface DraftOrder {
  id: string;
  items: any[];
  tableId?: string;
  tableName?: string;
  guestCount: number;
  subtotal: number;
  discount: number;
  total: number;
  notes: string[];
  createdAt: string;
  updatedAt: string;
}

export interface DraftOrdersState {
  draftOrders: DraftOrder[];
  draftOrderCount: number;

  // Actions
  addDraftOrder: (order: DraftOrder) => string;
  updateDraftOrder: (id: string, updates: Partial<DraftOrder>) => void;
  removeDraftOrder: (id: string) => void;
  clearDraftOrders: () => void;
  getDraftOrderCount: () => number;
}

export const useDraftOrdersStore = create<DraftOrdersState>()(
  persist(
    (set, get) => ({
      draftOrders: [],
      draftOrderCount: 0,

      addDraftOrder: (order) => {
        const id = order.id || crypto.randomUUID();
        const newOrder = { ...order, id, createdAt: new Date().toISOString() };
        set((state) => ({
          draftOrders: [...state.draftOrders, newOrder],
          draftOrderCount: state.draftOrders.length + 1,
        }));
        return id;
      },
      updateDraftOrder: (id, updates) => set((state) => ({
        draftOrders: state.draftOrders.map((o) =>
          o.id === id ? { ...o, ...updates, updatedAt: new Date().toISOString() } : o
        ),
      })),
      removeDraftOrder: (id) => set((state) => ({
        draftOrders: state.draftOrders.filter((o) => o.id !== id),
        draftOrderCount: state.draftOrders.length - 1,
      })),
      clearDraftOrders: () => set({ draftOrders: [], draftOrderCount: 0 }),
      getDraftOrderCount: () => get().draftOrders.length,
    }),
    {
      name: 'draft-orders-storage',
    }
  )
);

export function useDraftOrdersCount() {
  return useDraftOrdersStore((state) => state.draftOrderCount);
}

export default useDraftOrdersStore;
