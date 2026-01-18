import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface ReceiptState {
  dailySequence: number;
  generateReceiptNumber: () => string;
}

export const useReceiptStore = create<ReceiptState>()(
  persist(
    (set, get) => ({
      dailySequence: 10001,

      generateReceiptNumber: () => {
        const date = new Date();
        const year = date.getFullYear();
        const month = String(date.getMonth() + 1).padStart(2, '0');
        const day = String(date.getDate()).padStart(2, '0');
        const dateStr = `${year}${month}${day}`;

        const sequence = get().dailySequence;
        const number = `FAC${dateStr}${sequence}`;
        set({ dailySequence: sequence + 1 });
        return number;
      }
    }),
    {
      name: 'receipt-storage',
      partialize: (state) => ({ dailySequence: state.dailySequence })
    }
  )
);

 
