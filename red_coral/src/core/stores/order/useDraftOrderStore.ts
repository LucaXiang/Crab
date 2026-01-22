import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { DraftOrder, CartItem } from '@/core/domain/types';

interface DraftOrderState {
  draftOrders: DraftOrder[];
  saveDraft: (draft: DraftOrder) => void;
  restoreDraft: (id: string) => CartItem[];
  deleteDraft: (id: string) => void;
}

export const useDraftOrderStore = create<DraftOrderState>()(
  persist(
    (set, get) => ({
      draftOrders: [],

      saveDraft: (draft: DraftOrder) => {
        set((state) => ({
          draftOrders: [draft, ...state.draftOrders]
        }));
      },

      restoreDraft: (id: string) => {
        const draft = get().draftOrders.find(d => d.order_id === id);
        if (draft) {
          set((state) => ({
            draftOrders: state.draftOrders.filter(d => d.order_id !== id)
          }));
          return draft.items;
        }
        return [];
      },

      deleteDraft: (id: string) => {
        set((state) => ({
          draftOrders: state.draftOrders.filter(d => d.order_id !== id)
        }));
      }
    }),
    {
      name: 'draft-orders-storage',
      partialize: (state) => ({ draftOrders: state.draftOrders })
    }
  )
);

// Selectors
export const useDraftOrders = () => useDraftOrderStore((state) => state.draftOrders);
export const useDraftOrdersCount = () => useDraftOrderStore((state) => state.draftOrders.length);

 
