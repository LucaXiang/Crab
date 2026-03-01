import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { DraftOrder, CartItem } from '@/core/domain/types';

interface DraftOrderState {
  draftOrders: DraftOrder[];
  saveDraft: (draft: DraftOrder) => void;
  restoreDraft: (id: number) => CartItem[];
  deleteDraft: (id: number) => void;
}

export const useDraftOrderStore = create<DraftOrderState>()(
  persist(
    (set, get) => ({
      draftOrders: [] as DraftOrder[],

      saveDraft: (draft: DraftOrder) => {
        set((state) => ({
          draftOrders: [draft, ...state.draftOrders]
        }));
      },

      restoreDraft: (id: number) => {
        const draft = get().draftOrders.find(d => d.order_id === id);
        if (draft) {
          set((state) => ({
            draftOrders: state.draftOrders.filter(d => d.order_id !== id)
          }));
          return draft.items;
        }
        return [];
      },

      deleteDraft: (id: number) => {
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

 
