/**
 * Held Orders Store
 * Manages held orders (orders being worked on but not yet submitted)
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface HeldOrder {
  id: string;
  items: any[];
  tableId?: string;
  tableName?: string;
  guestCount: number;
  notes: string[];
  createdAt: string;
  updatedAt: string;
}

export interface HeldOrdersState {
  heldOrders: HeldOrder[];
  heldOrderCount: number;

  // Actions
  addHeldOrder: (order: HeldOrder) => string;
  updateHeldOrder: (id: string, updates: Partial<HeldOrder>) => void;
  removeHeldOrder: (id: string) => void;
  clearHeldOrders: () => void;
  getHeldOrderCount: () => number;
}

export const useHeldOrdersStore = create<HeldOrdersState>()(
  persist(
    (set, get) => ({
      heldOrders: [],
      heldOrderCount: 0,

      addHeldOrder: (order) => {
        const id = order.id || crypto.randomUUID();
        const newOrder = { ...order, id, createdAt: new Date().toISOString() };
        set((state) => ({
          heldOrders: [...state.heldOrders, newOrder],
          heldOrderCount: state.heldOrders.length + 1,
        }));
        return id;
      },
      updateHeldOrder: (id, updates) => set((state) => ({
        heldOrders: state.heldOrders.map((o) =>
          o.id === id ? { ...o, ...updates, updatedAt: new Date().toISOString() } : o
        ),
      })),
      removeHeldOrder: (id) => set((state) => ({
        heldOrders: state.heldOrders.filter((o) => o.id !== id),
        heldOrderCount: state.heldOrders.length - 1,
      })),
      clearHeldOrders: () => set({ heldOrders: [], heldOrderCount: 0 }),
      getHeldOrderCount: () => get().heldOrders.length,
    }),
    {
      name: 'held-orders-storage',
    }
  )
);

export function useHeldOrdersCount() {
  return useHeldOrdersStore((state) => state.heldOrderCount);
}

export default useHeldOrdersStore;
